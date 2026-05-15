"use strict";

const assets = require("./install/assets");
const download = require("./install/download");
const installer = require("./install/installer");
const platform = require("./install/platform");

module.exports = {
  ...assets,
  ...download,
  ...installer,
  ...platform,
};
