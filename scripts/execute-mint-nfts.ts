import * as dotenv from 'dotenv'
import { MnemonicKey, LCDClient, MsgExecuteContract } from '@terra-money/feather.js';
import * as fs from 'fs';
import * as _ from 'lodash';

dotenv.config();

(async () => {
    try {

        // Create the LCD Client to interact with the blockchain
        const lcd = new LCDClient({
            'pisco-1': {
                lcd: 'http://192.168.2.101:1317/',
                chainID: 'pisco-1',
                gasAdjustment: 1.75,
                gasPrices: { uluna: 0.015 },
                prefix: 'terra'
            }
        })
        const contractAdress = fs.readFileSync('./scripts/.nft_minter_contract_address.log').toString();

        // Create this to be able to iterate over it with for await
        const array = new Array(50).fill(0);
        let index = 0;
        for await (const _ of array) {
            const mk = new MnemonicKey({ mnemonic: process.env.MNEMONIC, index: index });
            const wallet = lcd.wallet(mk);
            const accAddress = wallet.key.accAddress("terra");
            const tx = await wallet.createAndSignTx({
                msgs: [
                    new MsgExecuteContract(
                        accAddress,
                        contractAdress,
                        { mint: {} },
                    ),
                ],
                chainID: "pisco-1",
            });
            let result = await lcd.tx.broadcastSync(tx, "pisco-1");
            console.log(`NFT Minted for address: ${accAddress} with TXHAsh ${result.txhash}`);
            index++;
        }
    }
    catch (e) {
        console.log(e)
    }
})()