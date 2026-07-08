<?php

require_once __DIR__ . '/src/Helper.php';

use App\Helper;

// Wrapped in a function, not incidental: a bare top-level call is never
// attributed to an enclosing symbol (no call_sites row at all), confirmed
// empirically against this exact fixture before this wrapper was added —
// same lesson js/main.js's own README note already documents for JS.
function run(): void
{
    // Inlined, not `$helper = new Helper(); $helper->greet(...)`: scip-php's
    // Types::type() has no local-variable data-flow analysis (a plain
    // Variable only resolves when named "this"), so an intermediate
    // variable would leave the ->greet() call permanently unresolved by
    // the real indexer. It does resolve a `new` expression inline.
    echo (new Helper())->greet("world");
}

run();
