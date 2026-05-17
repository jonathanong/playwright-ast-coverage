import express from "express";

const common = express.Router();

common.get("/ping", ping);

export default common;

function ping() {}
