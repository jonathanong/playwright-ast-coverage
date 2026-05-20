import express from "express";

const app = express();

app.get("/api/users/:id", handler);
