const { nodeAnvilNew, nodeAnvilHandleRequest } = require('./bin/index.node');

class NodeAnvil {
    constructor() {
        this.instance = nodeAnvilNew();
    }

    async request(req) {
        const resp = await nodeAnvilHandleRequest.call(this.instance, JSON.stringify(req));
        return JSON.parse(resp).result;
    }
}

module.exports = NodeAnvil;
