This directory contains some benchmarks comparing different implementations of various functions.
This is the whole reason the application is structured as a thin wrapper around a library implementing all
functionality: it's not possible to benchmark a binary with criterion. You _must_ benchmark a library.
