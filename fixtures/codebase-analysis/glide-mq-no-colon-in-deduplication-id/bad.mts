queue.add("send-email", data, {
  deduplication: {
    id: "email:123",
  },
});
