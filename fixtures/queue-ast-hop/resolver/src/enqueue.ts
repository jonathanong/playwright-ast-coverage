import { queue } from "@queues";
import { direct } from "./direct.ts";

queue.add("run", direct);
