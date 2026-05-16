import { Hono } from "hono";

const app = new Hono().basePath("/api");
const posts = new Hono();
const commentPath = `/posts/:id/comments`;

posts.get("/:id", getPost);
app.route("/posts", posts);
app.on("POST", "/posts", createPost);
app.on(["GET", "POST"], [commentPath, "/posts/:id/likes"], createPost);

function getPost() {}
function createPost() {}
