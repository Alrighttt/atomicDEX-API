This is a WIP branch for adding CC token support into mm2. Nearly all of the code I've written here is likely to change. I would not recommend using this for anything as of now. 

The next step here is to produce CC scriptsigs and from there, full CC spends. I first need to hack the parity bitcoin library we are utilizing or find an alternative. I am facing an issue whereas the `script` objects are validated by parity's transaction builder. It has no concept of "CC" or the "cc" OP_CODE. This validation needs to be bypassed.

So far it supports:

querying SPV servers for unspent token UTXOs - currently no API call for this as I did not think it an important thing for a user to need. This will be used for creating transactions. 

querying SPV servers for token creation data:
```
curl --url "http://127.0.0.1:7783" --data '{"userpass":"curnoi209c0909c2ijcouinfoirncosnrco","method":"tokens", "submethod":"info","tokenid":"ecb4084ff33a58ec6f8feba23c952fa10185d7e426c02fdedfec597840484e28","coin":"RICK"}' -s| jq
{
  "Create": {
    "tokenid": "ecb4084ff33a58ec6f8feba23c952fa10185d7e426c02fdedfec597840484e28",
    "creator": "026943ec773d7a273d56d50546583ded7bc3521a731cc9d4dba273e67c721d2832",
    "name": "tt1",
    "supply": 100000000,
    "description": "someNFT"
  }
}
```
or transfers:
```
curl --url "http://127.0.0.1:7783" --data '{"userpass":"curnoi209c0909c2ijcouinfoirncosnrco","method":"tokens", "submethod":"info","tokenid":"943203a6cebeef402792d07a46486b56cd14f30e8bc0310cbab379b6f3a24aea","coin":"RICK"}' -s|jq
{
  "Transfer": {
    "tokenid": "b624b3a2d25042109a6b346c40e78bb852596785666311de81d6df8a6211c78b",
    "received": 254,
    "recepient": "02480688f3bd5d455481c63607d5ab2ddbe3f9ba407b7dd91b68f7e983e8b8c2e4"
  }
}
```

querying SPV servers for balances of arbitrary pubkey and tokens:
```
curl --url "http://127.0.0.1:7783" --data '{"userpass":"curnoi209c0909c2ijcouinfoirncosnrco","method":"tokens", "submethod":"balance","tokenid":"ecb4084ff33a58ec6f8feba23c952fa10185d7e426c02fdedfec597840484e28","coin":"RICK", "pubkey":"026943ec773d7a273d56d50546583ded7bc3521a731cc9d4dba273e67c721d2832"}' -s|jq
{
  "result": "success",
  "balance": 100000000
}
```

querying SPV servers for your own balance for a given token:
```
curl --url "http://127.0.0.1:7783" --data '{"userpass":"curnoi209c0909c2ijcouinfoirncosnrco","method":"tokens", "submethod":"balance","tokenid":"ecb4084ff33a58ec6f8feba23c952fa10185d7e426c02fdedfec597840484e28","coin":"RICK"}' -s|jq
{
  "result": "success",
  "balance": 0
}
```
