<?php
namespace Nuanced;

final class FizzBuzz {
  public static function fizzbuzz(int $n): string {
    if ($n % 15 === 0) return "fizzbuzz";
    if ($n % 3 === 0) return "fizz";
    if ($n % 5 === 0) return "buzz";
    return strval($n);
  }
}
