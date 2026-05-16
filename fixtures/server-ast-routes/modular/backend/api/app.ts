import express from "express";
import { admin } from "./admin";
import { users } from "./users";

const app = express();

app.use("/api", requireAuth, users);
app.use(admin);

function requireAuth() {}
