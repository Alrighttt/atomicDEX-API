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

use rpc::v1::types::{H256 as H256Json, H264 as H264Json, H160 as H160Json};

use coins::MmCoinEnum::UtxoCoin;
use coins::utxo::utxo_standard::UtxoStandardCoin;
use coins::utxo::{UtxoTx, FeePolicy};

use serialization::{deserialize, serialize,Deserializable, Reader, Serializable, Stream};
use serialization::bytes::Bytes;
use hex;
use std::convert::TryInto;

use primitives::{hash};
use hash::{H256};

// FIXME Above imports are a mess, can get rid of a good amount of them

pub async fn z_router(ctx: MmArc, req: Json) -> Result<Response<Vec<u8>>, String> {
	let ticker = try_s!(req["coin"].as_str().ok_or("ztools: No 'coin' field")).to_owned();
    let coin = match lp_coinfind(&ctx, &ticker).await {
        Ok(Some(t)) => t,
        Ok(None) => return ERR!("ztools: No such coin: {}", ticker),
        Err(err) => return ERR!("ztools: !lp_coinfind({}): {}", ticker, err),
    };

    // FIXME Alright 
    //     at least use the is assetchain flag
    let utxo_coin = match coin {
        UtxoCoin(standard) => standard,
         _ => return ERR!("ztools: Must be UTXO coin, not: {}", ticker),
    };

    let submethod = match req["submethod"].clone() {
        Json::String(submethod) => submethod,
        _ => return ERR!("ztools: no submethod field: {}", ticker),
    };

	match submethod.as_str() {
    	"sign" => return(sign(ctx, req, utxo_coin)).await,
   		_ => return ERR!("ztools: submethod:{} does not exist", submethod),
	};
}

use script::TransactionInputSigner;
use chain::Transaction;
use script::{Opcode,Builder,UnsignedTransactionInput};
use coins::utxo::utxo_common::payment_script;
use coins::utxo::Public;
use chain::constants::SEQUENCE_FINAL;
use chain::OutPoint;
use coins::utxo::utxo_common::p2sh_spend;

#[derive(Serialize, Deserialize)]
struct InfoReq {
    rawhex: String,
    maker_pub: String,
    locktime: u32,
    inputs: UserInputInput,
    secret_hash: H160Json
}

#[derive(Serialize, Deserialize)]
struct UserInputInput {
    sequence: u32,
    amount: u64,
    prev_txid: H256Json,
    prev_index: u32
}


impl From<UserInputInput> for UnsignedTransactionInput {
    fn from(i: UserInputInput) -> Self {
        UnsignedTransactionInput {
            previous_output: OutPoint { 
                               hash: i.prev_txid.reversed().into(),
                               index: i.prev_index},
            sequence: i.sequence,
            amount: i.amount,
        }
    }
}


pub async fn sign(_ctx: MmArc, req: Json, utxo_coin: UtxoStandardCoin) -> Result<Response<Vec<u8>>, String> {
    let req: InfoReq = try_s!(json::from_value(req));

    // FIXME can generalize maker/taker here to allow for either
    let maker_pub = try_s!(Public::from_slice(hex::decode(req.maker_pub).unwrap().as_slice()));
    let taker_pub = utxo_coin.as_ref().key_pair.public();
    let secret_hash :Vec<u8> = req.secret_hash.into();

    let mut tx :Transaction = deserialize(try_s!(hex::decode(req.rawhex)).as_slice()).unwrap();
    let mut tx_unsigned : TransactionInputSigner = tx.clone().into();

    let inputs = vec![UnsignedTransactionInput::from(req.inputs)];

    tx_unsigned.inputs = inputs;

    let script_data = Builder::default().push_opcode(Opcode::OP_1).into_script();
    let redeem_script = payment_script(
        req.locktime,
        secret_hash.as_slice(),
        taker_pub,
        &maker_pub
    );

    println!("REDEEM: {}", redeem_script);
    
    let signed_input = try_s!(p2sh_spend(
        &tx_unsigned,
        0,
        &utxo_coin.as_ref().key_pair,
        script_data,
        redeem_script.into(),
        utxo_coin.as_ref().conf.signature_version,
        utxo_coin.as_ref().conf.fork_id
    ));

    println!("SIGNED VIN: {:?}", signed_input);

    tx.inputs = vec![signed_input];

    println!("SIGNED TX: {:?}", tx);
    println!("SERIAL {:?}", serialize(&tx));

    // FIXME WIP this can easily return something, just pushing as is for the moment 
    return ERR!("ztools: does not appear to be a tokenid or token transfer!!!");
}
