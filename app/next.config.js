const CopyPlugin = require("copy-webpack-plugin");
const withTM = require("next-transpile-modules")(["@nothing.app/skia-backend"]);

module.exports = withTM({
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
});
