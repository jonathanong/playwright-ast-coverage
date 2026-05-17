import express from "express";
import users from "@routes/users";
import common from "./routes/common";

const app = express();

app.use("/api", users);
app.use("/cjs", common);
