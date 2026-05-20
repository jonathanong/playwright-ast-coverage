import express from "express";
import request from "supertest";

const app = express();

app.get("/api/test-only", handler);
request(app).get("/api/users/42");
