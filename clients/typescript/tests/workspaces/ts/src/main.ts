import { fizzbuzz } from "./fizzbuzz.js";
import { add, Greeter } from "./util.js";

const total = add(2, 3);
const g = new Greeter("Nuanced");
const series = Array.from({ length: 20 }, (_, i) => fizzbuzz(i + 1));
console.log(total, g.greet(), series);
