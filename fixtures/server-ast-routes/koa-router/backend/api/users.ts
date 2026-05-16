import Router from "@koa/router";

const api = new Router({ prefix: "/api" });
const users = new Router();

users.get("user", "/:id", getUser);
users.del("/:id", deleteUser);
api.use("/users", users.routes());

function getUser() {}
function deleteUser() {}
