import { Router } from "express";

const users = Router();

users.get("/users/:id", getUser);

export default users;

function getUser() {}
