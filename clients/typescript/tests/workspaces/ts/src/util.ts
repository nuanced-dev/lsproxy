export function add(a: number, b: number): number { return a + b; }

export class Greeter {
  constructor(public name: string) {}
  greet(): string { return `Hello, ${this.name}!`; }
}
