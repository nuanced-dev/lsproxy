<?php
require __DIR__ . '/vendor/autoload.php';

use Nuanced\FizzBuzz;
use function Nuanced\add;
use Nuanced\Greeter;

$total = add(2, 3);
$g = new Greeter("Nuanced");
$series = array_map(fn($i) => FizzBuzz::fizzbuzz($i), range(1, 20));
echo $total . " " . $g->greet() . " [" . implode(", ", $series) . "]";
