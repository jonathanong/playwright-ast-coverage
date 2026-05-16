import express from "express";

export const admin = express.Router();

admin.get("/admin", getAdmin);

function getAdmin() {}
