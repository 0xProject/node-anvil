const { describe, expect, test } = require('@jest/globals');

const NodeAnvil = require('.');
const ganache = require('ganache');

/**
    contract Hello {
        function hello() public pure returns (string memory) {
            return "Hello world";
        }
      function add(uint256 val) public pure returns (uint256) {
          return 1 + val;
      }
    }
 */
const helloByteCode =
    '0x608060405234801561001057600080fd5b5061024d806100206000396000f3fe608060405234801561001057600080fd5b50600436106100365760003560e01c80631003e2d21461003b57806319ff1d211461006b575b600080fd5b610055600480360381019061005091906100e8565b610089565b604051610062919061017b565b60405180910390f35b610073610096565b6040516100809190610159565b60405180910390f35b6000816001019050919050565b60606040518060400160405280600b81526020017f48656c6c6f20776f726c64000000000000000000000000000000000000000000815250905090565b6000813590506100e281610200565b92915050565b6000602082840312156100fa57600080fd5b6000610108848285016100d3565b91505092915050565b600061011c82610196565b61012681856101a1565b93506101368185602086016101bc565b61013f816101ef565b840191505092915050565b610153816101b2565b82525050565b600060208201905081810360008301526101738184610111565b905092915050565b6000602082019050610190600083018461014a565b92915050565b600081519050919050565b600082825260208201905092915050565b6000819050919050565b60005b838110156101da5780820151818401526020810190506101bf565b838111156101e9576000848401525b50505050565b6000601f19601f8301169050919050565b610209816101b2565b811461021457600080fd5b5056fea2646970667358221220f802f3f4f281d6d56931c6c2a4f1a4badc24f28d15e272ee9cf77835aa24306864736f6c634300060c0033';

testMultiple = async (i1, i2) => {
    let i1Block;
    let i2Block;
    i1Block = await i1.request({ method: 'eth_blockNumber', params: [] });
    expect(i1Block).toBe('0x0');
    await i1.request({ method: 'evm_mine', params: [] });
    i1Block = await i1.request({ method: 'eth_blockNumber', params: [] });
    expect(i1Block).toBe('0x1');

    i2Block = await i2.request({ method: 'eth_blockNumber', params: [] });
    expect(i2Block).toBe('0x0');
    await i2.request({ method: 'evm_mine', params: [] });
    i2Block = await i2.request({ method: 'eth_blockNumber', params: [] });
    expect(i2Block).toBe('0x1');

    i1Block = await i1.request({ method: 'eth_blockNumber', params: [] });
    expect(i1Block).toBe('0x1');
};

testDeployCall = async (i, code, callCode) => {
    const accounts = await i.request({ method: 'eth_accounts', params: [] });
    console.log({ accounts });
    await i.request({
        method: 'eth_sendTransaction',
        params: [
            {
                from: accounts[0],
                data: code,
                value: '0x0',
                gas: '0x76c000',
            },
        ],
    });
    const deployedCode = await i.request({
        method: 'eth_getCode',
        params: ['0x5fbdb2315678afecb367f032d93f642f64180aa3', 'latest'],
    });
    console.log({ deployedCode });
    // TODO remove this requirement from NodeAnvil settings
    await i.request({ method: 'evm_mine', params: [] });
    const callResult = await i.request({
        method: 'eth_call',
        params: [
            {
                to: '0x5fbdb2315678afecb367f032d93f642f64180aa3',
                data: '0x19ff1d21',
            },
        ],
    });
    console.log(callResult);
};

describe('ganache', () => {
    const opts = {
        logging: { logger: { log: () => {} } },
        mnemonic: 'test test test test test test test test test test test junk',
    };
    // const opts = { mnemonic: 'test test test test test test test test test test test junk' };
    test('multiple instances', async () => {
        await testMultiple(ganache.provider(opts), ganache.provider(opts));
    });
    test('deploy call', async () => {
        await testDeployCall(ganache.provider(opts), helloByteCode, '');
    });
});

describe('nodeanvil', () => {
    test('multiple instances', async () => {
        await testMultiple(new NodeAnvil(), new NodeAnvil());
    });

    test('deploy call', async () => {
        await testDeployCall(new NodeAnvil(), helloByteCode, '');
    });
});
