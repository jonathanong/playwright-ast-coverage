import express from "express";

const admin = express.Router();

admin.get("/admin", getAdmin);

export default admin;

function getAdmin() {}
