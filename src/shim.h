#pragma once
#include "rust/cxx.h"
#include <string>

struct SolverOptions;

rust::String solve_wrapper(rust::String input_json, const SolverOptions& options);
