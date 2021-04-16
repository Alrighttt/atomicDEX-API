use coins::lp_coinfind;

use gstuff::{ERR, ERRL,try_s};

use rustc_hex::ToHex;
use common::mm_ctx::MmArc;
use http::{Response};
use serde_json::{self as json, Value as Json};
use serde::{Serialize,Deserialize};
use std::{io};

use cryptoconditions::{Threshold, Secp256k1, Eval};
use rustc_hex::{FromHex};
use secp256k1::{PublicKey};

use futures::compat::Future01CompatExt;

use rpc::v1::types::{H256 as H256Json};

use coins::MmCoinEnum::UtxoCoin;
use coins::utxo::utxo_standard::UtxoStandardCoin;
use coins::utxo::{UtxoTx, FeePolicy};

use serialization::{deserialize, Deserializable, Reader, Serializable, Stream};
use serialization::bytes::Bytes;
use hex;
use std::convert::TryInto;

use primitives::{hash};
use hash::{H256};

// FIXME Above imports are a mess, can get rid of a good amount of them

#[derive(Serialize, Deserialize)]
struct TokenTransferReq {
	tokenid: String,
	dest_pubkey: String,
	dest_amount: u64,
}

#[derive(Serialize, Deserialize)]
struct TokenBalanceResp {
	result: String,
    balance: u64
}

#[derive(Serialize, Deserialize)]
struct TokenInfoReq {
	tokenid: String
}

#[derive(Serialize, Deserialize)]
enum TokenInfoResp {
    Transfer {
        tokenid: String,
        received: u64,
        recepient: String,
    },
    Create {
        tokenid: String,
        creator: String,
        name: String,
        supply: u64,
        description: String,
    },
}


#[derive(Clone, Debug, PartialEq)]
pub struct TokenCreateOpReturn {
    pub op_code: u8,
    pub opret_len: u8,
    pub eval_code: u8,
    pub funcid: u8,
    pub pubkey: Bytes,
    pub name: String,
    pub description: String

}

impl Deserializable for TokenCreateOpReturn {
    fn deserialize<T: io::Read>(reader: &mut Reader<T>) -> Result<Self, serialization::Error> where Self: Sized {
        let op_code = reader.read()?;
        let opret_len = reader.read()?;
        let eval_code = reader.read()?;
        let funcid = reader.read()?;
        let pubkey = reader.read()?;
        let name = reader.read()?;
        let description = reader.read()?;

        Ok(TokenCreateOpReturn {
            op_code,
            opret_len,
            eval_code,
            funcid,
            pubkey,
            name,
            description,
        })
    }
}

impl Serializable for TokenCreateOpReturn {
    fn serialize(&self, s: &mut Stream) {
        s.append(&self.op_code);
        s.append(&self.opret_len);
        s.append(&self.eval_code);
        s.append(&self.funcid);
        s.append(&self.pubkey);
        s.append(&self.name);
        s.append(&self.description);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TokenTransferOpReturn {
    pub op_code: u8,
    pub opret_len: u8,
    pub eval_code: u8,
    pub funcid: u8,
    pub tokenid: H256,
    pub pk_amount: u8,
    pub pubkey: Bytes,
}

impl Deserializable for TokenTransferOpReturn {
    fn deserialize<T: io::Read>(reader: &mut Reader<T>) -> Result<Self, serialization::Error> where Self: Sized {
        let op_code = reader.read()?;
        let opret_len = reader.read()?;
        let eval_code = reader.read()?;
        let funcid = reader.read()?;
        let tokenid = reader.read()?;
        let pk_amount = reader.read()?;
        let pubkey = reader.read()?;

        Ok(TokenTransferOpReturn {
            op_code,
            opret_len,
            eval_code,
            funcid,
            tokenid,
            pk_amount,
            pubkey,
        })
    }
}

impl Serializable for TokenTransferOpReturn {
    fn serialize(&self, s: &mut Stream) {
        s.append(&self.op_code);
        s.append(&self.opret_len);
        s.append(&self.eval_code);
        s.append(&self.funcid);
        s.append(&self.tokenid);
        s.append(&self.pk_amount);
        s.append(&self.pubkey);
    }
}

pub enum TokenOpRet {
    Transfer(TokenTransferOpReturn),
    Create(TokenCreateOpReturn),
}

pub async fn tokens_router(ctx: MmArc, req: Json) -> Result<Response<Vec<u8>>, String> {
	let ticker = try_s!(req["coin"].as_str().ok_or("tokens: No 'coin' field")).to_owned();
    let coin = match lp_coinfind(&ctx, &ticker).await {
        Ok(Some(t)) => t,
        Ok(None) => return ERR!("tokens: No such coin: {}", ticker),
        Err(err) => return ERR!("tokens: !lp_coinfind({}): {}", ticker, err),
    };

    // FIXME Alright 
    //     need a isCC flag or at least use the is assetchain flag
    let utxo_coin = match coin {
        UtxoCoin(standard) => standard,
         _ => return ERR!("tokens: Must be UTXO coin, not: {}", ticker),
    };

    let submethod = match req["submethod"].clone() {
        Json::String(submethod) => submethod,
        _ => return ERR!("tokens: no submethod field: {}", ticker),
    };

	match submethod.as_str() {
    	"info" => return(token_info(ctx, req, utxo_coin)).await,
    	"transfer" => return(token_withdraw(ctx, req, utxo_coin)).await,
        "balance" => return(my_balance(ctx, req, utxo_coin)).await,
   		_ => return ERR!("tokens: submethod:{} does not exist", submethod),
	};
}

// TODO Alright
// tokencreate, check outputs for CC scriptpubkeys
//     check that all CC outputs go to creator pubkey and no others 
//     it is possible for a valid tokencreate to violate this, but it would require custom komodod code
// check that at least one input is signed by creator pubkey

// tokentransfer, grab the sender pubkey and display it
//     a bit difficult as there are many possible edge cases if someone makes custom komodod code

// this whole thing feels quite janky(especially the API response)
// will come back to it later if anyone actually begins to use this
pub async fn token_info(_ctx: MmArc, req: Json, utxo_coin: UtxoStandardCoin) -> Result<Response<Vec<u8>>, String> {
    let req: TokenInfoReq = try_s!(json::from_value(req));
    let tokenid: H256Json = try_s!(hex::decode(req.tokenid.clone())).as_slice().into();

    let rpc_client = &utxo_coin.as_ref().rpc_client;

    let verbose_tx = try_s!(rpc_client.get_verbose_transaction(tokenid).compat().await);
    let tx: UtxoTx = try_s!(deserialize(verbose_tx.hex.as_slice()).map_err(|e| ERRL!("{:?}", e)));

    match tx.outputs.last() {
            None => return ERR!("tokens: {} does not appear to be a token!", req.tokenid),
            Some(_i) => (),
    };

    let opret_spk = &tx.outputs.last().unwrap().script_pubkey;

    let decode = decode_token_opret(opret_spk);
    match decode {
        Ok(e) => match e {
            TokenOpRet::Create(opret_data) => {
                let token_amount = &tx.outputs[1].value;
                let res = TokenInfoResp::Create {
                    creator: opret_data.pubkey.to_hex(),
                    name: opret_data.name,
                    tokenid: req.tokenid.to_string(),
                    supply: *token_amount,
                    description: opret_data.description,
                };
                let res = json::to_vec(&res).unwrap();
                return Ok(Response::builder().body(res).unwrap());
            },
            TokenOpRet::Transfer(opret_data) => {
                let token_amount = &tx.outputs[0].value;
                let res = TokenInfoResp::Transfer {
                    tokenid: opret_data.tokenid.to_hex(),
                    recepient: opret_data.pubkey.to_hex(),
                    received: *token_amount,
                };
                let res = json::to_vec(&res).unwrap();
                return Ok(Response::builder().body(res).unwrap());
            },
        },
        _=> (),
    }

    return ERR!("tokens: {} does not appear to be a tokenid or token transfer!!!", req.tokenid);
}

#[derive(Serialize, Deserialize)]
struct TokenBalanceReq {
    tokenid: String,
    pubkey: Option<String>,
}

fn cc_spk_1of1(eval_code: u8, pk:Vec<u8>) -> Vec<u8> {
    let cond = Threshold {
        threshold: 2,
        subconditions: vec![
            Threshold {
                threshold: 1,
                subconditions: vec![
                    Secp256k1 {
                        pubkey: PublicKey::parse_slice(&pk, None).unwrap(),
                        signature: None
                    }
                ]
            },
            Eval { code: vec![eval_code] }
        ]
    };
    let mut spk = cond.encode_condition();
    // FIXME Alright
    //     stupid manual serialization again
    //     particularly bad .unwrap()
    let push_len :u8 = spk.len().try_into().unwrap();
    spk.insert(0, push_len);
    spk.push(0xcc);

    spk    
}

pub fn decode_token_opret(opret_spk: &Bytes) -> Result<TokenOpRet, String> {
    let opret = opret_spk.clone().take();

    // check that this is an OP_RETURN (6a) and eval code f2
    if opret[0] != 0x6a || opret[2] != 0xf2 {
        return Err("tokens: {} does not appear to be a token OP_RETURN!!".to_string());
    }
    if opret[3] == 0x63 {
        // tokencreate 'c'
        let opret_data :TokenCreateOpReturn = try_s!(deserialize(opret_spk.as_slice()).map_err(|e| ERRL!("tokens: create op return derserialization failed {:?}", e)));
        return Ok(TokenOpRet::Create(opret_data));
    } else if opret[3] == 0x74 {
        // tokencreate 't'
        let opret_data :TokenTransferOpReturn = try_s!(deserialize(opret_spk.as_slice()).map_err(|e| ERRL!("tokens: create op return derserialization failed {:?}", e)));
        return Ok(TokenOpRet::Transfer(opret_data));
    }
    Err("tokens: unrecognized tokens funcid!".to_string())
}

pub async fn my_balance(ctx: MmArc, req: Json, utxo_coin: UtxoStandardCoin) ->  Result<Response<Vec<u8>>, String> {
    let req: TokenBalanceReq = try_s!(json::from_value(req));

    let pk = match req.pubkey {
        None => (&**ctx.secp256k1_key_pair().public()).to_vec(),
        Some(pk) => try_s!(pk.from_hex::<Vec<u8>>()),
    };

    let rpc_client = &utxo_coin.as_ref().rpc_client;
    let cc_spk = cc_spk_1of1(0xf2, pk);
    let utxos = try_s!(rpc_client.list_unspent_any(&cc_spk, 0x08).compat().await);

    // All user tokens are at this "cc_spk"(scriptPubKey)
    // it is possible they have many different tokens in this "cc_spk"
    // we must compare against "tokenid" field in vout[-1]
    // FIXME Alright
    //     as of tokensv1 tokencreate txes are not validated until their token outputs are spent
    //     we cannot trust these unvalidated UTXOs, they could be unspendable
    let mut balance = 0;
    for utxo in utxos.iter() {
        let txid: H256Json = hex::decode(utxo.outpoint.hash.to_reversed_str()).unwrap().as_slice().into();
        //let vout_index = utxo.outpoint.index;
        let vout_value = utxo.value;
        let verbose_tx = try_s!(rpc_client.get_verbose_transaction(txid).compat().await);
        let tx: UtxoTx = try_s!(deserialize(verbose_tx.hex.as_slice()).map_err(|e| ERRL!("{:?}", e)));

        let opret_spk = &tx.outputs.last().unwrap().script_pubkey;
        let decode = decode_token_opret(opret_spk);
        match decode {
            Ok(e) => match e {
                TokenOpRet::Create(_) => {
                    if tx.hash().to_reversed_str() == req.tokenid {
                        balance += vout_value;
                    }
                },
                TokenOpRet::Transfer(f) => {
                    if f.tokenid.to_string() == req.tokenid {
                        balance += utxo.value;
                    }
                },
            },
            _=> (), // if balances mismatch, this is a good place to debug
        }
    }

    let res = TokenBalanceResp {
        result: "success".to_string(),
        balance: balance,
    };

    let res = json::to_vec(&res).unwrap();
    Ok(Response::builder().body(res).unwrap())
}

use coins::utxo::rpc_clients::UnspentInfo;
use coins::utxo::rpc_clients::UtxoRpcClientEnum;
pub async fn cc_utxos(pk:Vec<u8>, eval_code: u8, rpc_client: &UtxoRpcClientEnum) -> Result<Vec<UnspentInfo>, String> {
    let cc_spk = cc_spk_1of1(eval_code, pk);
    let utxos = try_s!(rpc_client.list_unspent_any(&cc_spk, 0x08).compat().await);
    Ok(utxos)
}

// FIMXE the mm21 UTXO signer might be expecting inputs in a certain order, if this is the case, fix that here
pub async fn token_utxos(pk:Vec<u8>, tokenid: String, rpc_client: &UtxoRpcClientEnum) -> Result<Vec<UnspentInfo>, String> {
    let mut utxos = try_s!(cc_utxos(pk, 0xf2, rpc_client).await);
    let mut token_utxos :Vec<UnspentInfo> = Vec::new();
    for utxo in utxos.drain(..) {  
        let txid: H256Json = hex::decode(utxo.outpoint.hash.to_reversed_str()).unwrap().as_slice().into();
        let verbose_tx = try_s!(rpc_client.get_verbose_transaction(txid).compat().await);
        let tx: UtxoTx = try_s!(deserialize(verbose_tx.hex.as_slice()).map_err(|e| ERRL!("{:?}", e)));
        let opret_spk = &tx.outputs.last().unwrap().script_pubkey;
        let decode = decode_token_opret(opret_spk);
        match decode {
            Ok(e) => match e {
                TokenOpRet::Create(_) => {
                   if tx.hash().to_reversed_str() == tokenid.to_string() {
                        token_utxos.push(utxo);
                    }
                },
                TokenOpRet::Transfer(f) => {
                    if f.tokenid.to_string() == tokenid {
                        token_utxos.push(utxo);
                    }
                },
            },
            _=> (),
        }
    }
    Ok(token_utxos)
}


use chain::{TransactionOutput};
use coins::utxo::UtxoCommonOps;
use coins::utxo::ActualTxFee;
pub async fn token_withdraw(ctx: MmArc, req: Json, utxo_coin: UtxoStandardCoin) ->  Result<Response<Vec<u8>>, String> {
    let req: TokenTransferReq = try_s!(json::from_value(req));

    // FIXME we will need to full keypair for creating CC signatures via secp256k1 lib
    let pk = (&**ctx.secp256k1_key_pair().public()).to_vec(); 

    let rpc_client = &utxo_coin.as_ref().rpc_client;
    let utxos = try_s!(token_utxos(pk.clone(), req.tokenid.clone(), &rpc_client).await);

    let cc_script_pubkey :Bytes= cc_spk_1of1(0xf2, pk).into();

    let output = TransactionOutput { value: req.dest_amount, script_pubkey: cc_script_pubkey.clone() };
    let outputs = vec![output];

    let gas_fee = None;
    let fee = Some(ActualTxFee::Fixed(1 as u64));

    let (unsigned, _data) = try_s!(
        utxo_coin.generate_transaction(utxos.clone(), outputs, FeePolicy::SendExact, fee, gas_fee)
            .await
    );

    // FIXME stuck here until parity code is patched to ignore CC OP_CODE
    // in the generate_transaction line above we feed it CC UTXOs as inputs, but they are dropped because parity sees them as "BadOpCode"
    println!("TRANSCTION DEBUG: {:#?}", unsigned);

    let script = script::Script { data: cc_script_pubkey.into()};
    let mut count :usize = 0;
    for utxo in utxos.iter() {
        let sighash = unsigned.signature_hash(count, utxo.value, &script, script::SignatureVersion::Base, 1);
        count += 1;
    };

/*
TODO Alright
    create the condition object with a signature 
    https://github.com/Alrighttt/pycc-1/blob/b8c5d424de0dd6ef2197088248fccc812e46345c/cryptoconditions/src/condition.rs#L52

    encode to scriptsig via https://github.com/Alrighttt/pycc-1/blob/b8c5d424de0dd6ef2197088248fccc812e46345c/cryptoconditions/src/condition.rs#L183

    scriptsig - parity bitcoin script code will moan about these "BadOpcode" so need to hack the parity lib
    or somehow prevent it from trying to validate the script

    hopefully we can let mm21's typical utxo signer sign what it can - ( it seems to limited to one key at a time, so this may cause issues)
    then inject the CC scriptsigs after the fact. We can't validate transactions(nor should we as CC validation is complex and would require a 1:1 port to rust)
    so we'll just assume it is good and let the electrum server tell us if it is not 
*/


    let res = TokenBalanceResp {
        result: "NOT IMPLEMENTED".to_string(),
        balance: 666,
    };

    let res = json::to_vec(&res).unwrap();
    Ok(Response::builder().body(res).unwrap())
    
}