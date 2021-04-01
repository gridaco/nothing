const CopyPlugin = require("copy-webpack-plugin");

module.exports = {
  webpack(config) {
    config.node = {
      fs: "empty",
    };
    config.output.webassemblyModuleFilename = "static/wasm/[modulehash].wasm";

    config.plugins.push(
      new CopyPlugin({
        patterns: [
          {
            from: "../node_modules/canvaskit-wasm/bin/canvaskit.wasm",
          },
        ],
      })
    );

    return config;
  },
};
