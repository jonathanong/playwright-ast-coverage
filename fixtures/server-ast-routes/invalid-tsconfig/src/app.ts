import express from "express";

const app = express();

app.get("/health", health);

function health() {}
