export function add(a, b) {
  return a + b;
}

export class Greeter {
  constructor(name) {
    this.name = name;
  }
  greet() {
    return `Hello, ${this.name}!`;
  }
}
