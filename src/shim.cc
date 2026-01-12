#include "src/shim.h"
#include "jres_solver_service/src/main.rs.h"
#include "jres_solver/jres_solver.hpp"
#include <iostream>
#include <string>

rust::String solve_wrapper(rust::String input_json, const SolverOptions& options) {

    // Convert rust::String to std::string for null-termination
    std::string input_str(input_json);

    // Convert options
    JresSolverOptions c_options;
    c_options.timeLimit = options.time_limit;
    c_options.spotterMode = static_cast<JresSpotterMode>(options.spotter_mode);
    c_options.allowNoSpotter = options.allow_no_spotter;
    c_options.optimalityGap = options.optimality_gap;
    c_options.roleCouplingWeight = options.role_coupling_weight;
    c_options.rotationBeatWeight = options.rotation_beat_weight;

    // Convert input
    JresSolverInput* input = jres_input_from_json(input_str.c_str());
    if (!input) {
        return rust::String("{\"error\": \"Failed to parse input JSON via jres_input_from_json\"}");
    }

    // Solve
    JresSolverOutput* output;
    if (options.diagnose) {
        output = diagnose_race_schedule(input, &c_options);
    } else {
        output = solve_race_schedule(input, &c_options);
    }
    
    // Output to JSON
    char* json_out = jres_output_to_json(output);
    std::string result_str = json_out ? std::string(json_out) : "{\"error\": \"Failed to generate output JSON\"}";
    rust::String result(result_str);

    // Cleanup
    if (json_out) {
        free_json_string(json_out);
    }
    if (output) {
        free_jres_solver_output(output);
    }
    if (input) {
        free_jres_solver_input(input);
    }

    return result;
}
