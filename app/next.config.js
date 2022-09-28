const path = require("path");
const fs = require("fs");

const withTM = require("next-transpile-modules")(
  [
    "@nothing.app/react-core",
    // "@nothing.app/react-compact",
    // "@nothing.app/react",
    // "@nothing.app/react-state",
  ],
  {
    debug: false,
  }
);

module.exports = withTM({
  webpack: function (config, { isServer }) {
    config.module.rules.push({
      test: /\.(eot|woff|woff2|ttf|svg|png|jpg|gif)$/,
      use: {
        loader: "url-loader",
        options: {
          limit: 100000,
          name: "[name].[ext]",
        },
      },
    });
    config.module.rules.push({
      test: /\.md$/,
      use: "raw-loader",
    });
    config.module.rules.push({
      test: /\.ts(x?)$/,
      use: "babel-loader",
    });

    //
    // region react hoisting
    //

    //  https://www.npmjs.com/package/next-transpile-modules#i-have-trouble-with-duplicated-dependencies-or-the-invalid-hook-call-error-in-react
    if (isServer) {
      config.externals = ["react", ...config.externals];
    }

    // resolving gridaco/grida 's node_modules (as this being used as submodule package)
    let reactPath;
    const grida_react_path = path.resolve(
      __dirname,
      "../../../", // this file's placement under relative to grida's submodule package
      "node_modules",
      "react"
    );
    const nothing_react_path = path.resolve(
      __dirname,
      "../", // this fil'es placement under nothing's git repo
      "node_modules",
      "react"
    );
    // if this is a package of grida, use grida's node_modules
    if (fs.existsSync(grida_react_path)) {
      reactPath = grida_react_path;
    } else {
      reactPath = nothing_react_path;
    }
    console.log("reactPath", reactPath);
    config.resolve.alias["react"] = reactPath;

    //
    // endregion react hoisting
    //

    if (!isServer) {
      config.node = {
        fs: "empty",
      };
    }

    return config;
  },
});
