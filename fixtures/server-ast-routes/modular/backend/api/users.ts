import { Router } from "express";

const users = Router();

users.get("/:id", getUser);

export { users as members };

function getUser() {}
