import { Console, Effect } from "effect";

const program = Console.log("Hello world");

Effect.runSync(program);
