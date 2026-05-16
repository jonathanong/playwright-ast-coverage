import express from "express";

const app = express();
const router = express.Router();

app.get("/api/v1/users", listUsers);
app.get("/api/v1/users/:id", getUser);
app.route(`/api/v1/users/:id`).patch(updateUser).delete(deleteUser);

router.post("/api/v1/users", createUser);

function listUsers() {}
function getUser() {}
function updateUser() {}
function deleteUser() {}
function createUser() {}
