import express from "express";
import admin from "./admin";
import { members } from "./users";

const app = express();

app.use("/api", requireAuth, members);
app.use(admin);

function requireAuth() {}
