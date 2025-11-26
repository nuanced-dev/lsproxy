<?php
namespace Nuanced;

function add(int $a, int $b): int { return $a + $b; }

final class Greeter {
  private string $name;
  public function __construct(string $name) { $this->name = $name; }
  public function greet(): string { return "Hello, {$this->name}!"; }
}
