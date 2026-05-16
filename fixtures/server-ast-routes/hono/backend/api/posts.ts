import { Hono } from "hono";

const app = new Hono().basePath("/api");
const posts = new Hono();

posts.get("/:id", getPost);
app.route("/posts", posts);
app.on("POST", "/posts", createPost);

function getPost() {}
function createPost() {}
