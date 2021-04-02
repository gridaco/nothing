const withTM = require("next-transpile-modules")(["@nothing.app/skia-backend"]);

module.exports = withTM({
  webpack(config) {
    config.node = {
      fs: "empty",
    };
    return config;
  },
});
