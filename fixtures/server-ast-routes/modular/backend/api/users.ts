import express from "express";

export const users = express.Router();

users.get("/:id", getUser);

function getUser() {}
