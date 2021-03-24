const CopyWebpackPlugin = require("copy-webpack-plugin");
const path = require("path");
module.exports = {
  webpack: (config, { buildId, dev, isServer, defaultLoaders, webpack }) => {
    /**
     * README on https://www.npmjs.com/package/canvaskit-wasm
     */
    config.node = {
      fs: "empty",
    };
    config.plugins.push(
      new CopyWebpackPlugin({
        patterns: [
          {
            from: `${path.dirname(
              require.resolve(`${"canvaskit-wasm"}/package.json`)
            )}/bin/canvaskit.wasm`,
          },
        ],
      })
    );

    return config;
  },
};
