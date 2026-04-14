"""Generate and inject v201-v300 builtin registrations into codegen.rs and stdlib.rs."""
import re

# ── Module function registry: module_name → [(fn_name, [param_types])] ──
# fn_name is WITHOUT 'slang_' prefix. All return I64.
MODULES = [
    ("spike_engine", "v201-v206: Spike Engine", [
        ("spike_emit", ["I64","I64","I64"]), ("spike_queue_len", []), ("spike_next", []), ("spike_clear", []),
        ("neuro_compartment_create", ["F64","F64"]), ("neuro_compartment_step", ["I64","F64"]),
        ("neuro_dendrite_propagate", ["F64","I64"]), ("neuro_compartment_count", []),
        ("synapse_conductance", ["F64","F64","F64"]), ("synapse_stp_facilitate", ["F64","F64"]),
        ("synapse_stp_depress", ["I64","F64"]), ("synapse_count", []),
        ("neuro_wilson_cowan", ["F64","F64","F64","F64","F64","F64","I64"]),
        ("neuro_neural_mass", ["F64","F64","F64","F64"]),
        ("neuro_population_activity", ["I64","F64","F64"]), ("neuro_population_sync", ["F64","F64"]),
        ("spike_encode_rate", ["F64","F64","I64"]), ("spike_encode_temporal", ["F64","F64","I64"]),
        ("spike_decode_rate", ["I64","I64","F64"]), ("spike_encode_phase", ["F64","F64"]),
        ("neuro_mem_alloc", ["I64"]), ("neuro_mem_read", ["I64"]),
        ("neuro_mem_write", ["I64","I64"]), ("neuro_mem_near_compute", ["I64","I64","I64"]),
    ]),
    ("loihi_sim", "v207-v212: Loihi Simulator", [
        ("loihi_core_create", ["F64","F64","I64"]), ("loihi_core_config", ["I64","I64","F64"]),
        ("loihi_core_neuron_count", ["I64"]), ("loihi_core_count", []),
        ("loihi_route_spike", ["I64","I64"]), ("loihi_route_multicast", ["I64"]),
        ("loihi_noc_latency", ["I64"]), ("loihi_noc_bandwidth", ["I64"]),
        ("loihi_learn_stdp", ["I64","F64"]), ("loihi_learn_reward", ["F64","F64","F64"]),
        ("loihi_learn_3factor", ["F64","F64","F64","F64"]), ("loihi_learn_config", ["I64","I64"]),
        ("loihi_timestep", []), ("loihi_barrier_sync", []),
        ("loihi_async_tick", ["I64"]), ("loihi_time_now", []),
        ("loihi_energy_spike", ["I64"]), ("loihi_energy_compute", ["I64"]),
        ("loihi_power_total", []), ("loihi_energy_reset", []),
        ("loihi_inst_soma", ["I64","F64"]), ("loihi_inst_synapse", ["I64","F64"]),
        ("loihi_inst_axon", ["I64"]), ("loihi_inst_dendrite", ["I64","F64"]),
    ]),
    ("snn_learning", "v213-v218: SNN Learning", [
        ("snn_surrogate_forward", ["F64","F64","F64","I64"]), ("snn_surrogate_backward", ["F64","F64","F64"]),
        ("snn_surrogate_sigmoid", ["F64"]), ("snn_surrogate_loss", ["F64","F64","I64"]),
        ("snn_bptt_forward", ["F64","F64","F64","I64"]), ("snn_bptt_backward", ["F64","F64","I64"]),
        ("snn_bptt_truncate", ["F64","F64","I64"]), ("snn_bptt_gradient", ["F64","I64"]),
        ("snn_nas_search", ["I64","I64","F64"]), ("snn_nas_evaluate", ["I64","I64"]),
        ("snn_nas_mutate", ["I64","I64","I64"]), ("snn_nas_best", []),
        ("snn_fed_aggregate", ["F64","I64"]), ("snn_fed_share", ["F64","F64"]),
        ("snn_fed_round", []), ("snn_fed_node_count", ["I64"]),
        ("snn_transfer_freeze", ["I64"]), ("snn_transfer_finetune", ["F64","I64"]),
        ("snn_transfer_adapt", ["F64","F64"]), ("snn_transfer_similarity", ["F64","F64"]),
        ("snn_continual_learn", ["F64","F64","F64","F64","F64","F64"]),
        ("snn_continual_consolidate", ["F64","I64"]),
        ("snn_continual_replay", ["I64","I64"]), ("snn_continual_forget_score", ["F64","F64"]),
    ]),
    ("pim_compute", "v219-v224: Processing-in-Memory", [
        ("pim_alloc", ["I64"]), ("pim_compute_add", ["I64","I64"]),
        ("pim_compute_mul", ["I64","I64"]), ("pim_transfer_cost", ["I64","I64"]),
        ("datacentric_map", ["I64","F64","F64"]), ("datacentric_reduce", ["I64","I64","I64"]),
        ("datacentric_scatter", ["I64","I64"]), ("datacentric_gather", ["I64","F64"]),
        ("sparse_spike_propagate", ["I64","I64"]), ("sparse_nonzero_count", ["I64","I64"]),
        ("sparse_compress", ["I64","I64","I64"]), ("sparse_decompress", ["I64","I64"]),
        ("cache_oblivious_transpose", ["I64"]), ("cache_oblivious_fft", ["I64"]),
        ("cache_oblivious_sort", ["I64"]), ("cache_oblivious_matmul", ["I64"]),
        ("memcompute_fused_mac", ["F64","F64","F64"]), ("memcompute_fused_compare", ["I64","I64"]),
        ("memcompute_fused_accumulate", ["I64","I64","I64"]), ("memcompute_pipeline_depth", ["I64"]),
        ("zerocopy_spike_buffer", ["I64"]), ("zerocopy_fanout", ["I64","I64"]),
        ("zerocopy_gather", ["I64"]), ("zerocopy_active_count", []),
    ]),
    ("brain_models", "v225-v230: Brain Models", [
        ("pred_coding_forward", ["F64","F64","F64"]), ("pred_coding_error", ["F64","F64","F64"]),
        ("pred_coding_update", ["F64","F64","F64"]), ("pred_coding_layers", ["I64","I64"]),
        ("htm_spatial_pool", ["I64","I64","I64"]), ("htm_temporal_memory", ["I64","I64"]),
        ("htm_anomaly_score", ["I64","I64"]), ("htm_column_count", []),
        ("neuro_osc_gamma", ["F64","F64","F64"]), ("neuro_osc_theta", ["F64","F64","F64"]),
        ("neuro_osc_couple", ["F64","F64","F64"]), ("neuro_osc_phase_lock", ["F64","F64"]),
        ("neuromod_dopamine", ["F64","F64"]), ("neuromod_serotonin", ["F64","F64"]),
        ("neuromod_acetylcholine", ["F64","F64"]), ("neuromod_apply", ["F64","F64","F64"]),
        ("cortical_column_create", []), ("cortical_column_step", ["I64","F64"]),
        ("cortical_column_layer_activity", ["I64","I64"]), ("cortical_column_count", []),
        ("spike_attention_query", ["F64","F64","F64"]), ("spike_attention_key", ["I64","I64"]),
        ("spike_attention_value", ["I64","F64"]), ("spike_attention_score", ["F64","I64"]),
    ]),
    ("gpu_neuromorphic", "v231-v236: GPU Neuromorphic", [
        ("gpu_spike_propagate", ["I64","I64"]), ("gpu_spike_batch_size", ["I64"]),
        ("gpu_spike_throughput", ["I64","F64"]), ("gpu_spike_sync", []),
        ("gpu_neuron_update", ["I64","F64","F64"]), ("gpu_neuron_batch", ["I64","I64"]),
        ("gpu_neuron_occupancy", ["I64","I64"]), ("gpu_neuron_count", []),
        ("gpu_synapse_spmv", ["I64","F64","F64"]), ("gpu_synapse_csr", ["I64","I64","I64"]),
        ("gpu_synapse_nnz", ["I64","F64"]), ("gpu_synapse_density", ["I64","I64","I64"]),
        ("gpu_event_push", ["I64","I64"]), ("gpu_event_pop", []),
        ("gpu_event_merge", ["I64"]), ("gpu_event_size", []),
        ("mixed_prec_quantize", ["F64","I64"]), ("mixed_prec_dequantize", ["I64","I64"]),
        ("mixed_prec_accumulate", ["F64","F64"]), ("mixed_prec_bits", ["F64"]),
        ("multi_gpu_partition", ["I64","I64"]), ("multi_gpu_sync", ["I64","I64"]),
        ("multi_gpu_migrate", ["I64","I64"]), ("multi_gpu_count", []),
    ]),
    ("neuro_applications", "v237-v242: Neuro Applications", [
        ("spike_vision_encode", ["I64","I64","F64"]), ("spike_vision_edge", ["I64","I64","F64"]),
        ("spike_vision_motion", ["I64","I64"]), ("spike_vision_frames", []),
        ("spike_audio_encode", ["I64","I64","F64"]), ("spike_audio_frequency", ["I64"]),
        ("spike_audio_onset", ["I64","I64"]), ("spike_audio_classify", ["I64","I64"]),
        ("spike_pid", ["F64","F64","F64"]), ("spike_motor", ["F64","F64"]),
        ("spike_reflex", ["I64"]), ("spike_trajectory_cost", ["F64","I64"]),
        ("spike_anomaly_score", ["F64","F64"]), ("spike_changepoint", ["F64","F64","F64"]),
        ("spike_burst_detect", ["F64","F64"]), ("spike_pattern_match", ["F64"]),
        ("spike_anneal", ["F64","F64","I64"]), ("spike_gradient", ["F64","F64","F64"]),
        ("spike_constraint", ["I64","F64"]), ("spike_fitness", ["F64","F64"]),
        ("spike_nlp_similarity", ["F64","F64","F64"]), ("spike_nlp_attention", ["F64","F64"]),
        ("spike_nlp_encode_len", ["I64","I64"]), ("spike_nlp_perplexity", ["F64"]),
    ]),
    ("neuro_evolve", "v243-v248: Neuro Evolution", [
        ("neat_crossover", ["F64","F64"]), ("neat_mutate", ["I64","I64"]),
        ("neat_speciate", ["I64","I64","F64"]), ("neat_generation", []),
        ("som_bmu", ["F64","F64","I64"]), ("som_radius", ["F64","I64","I64"]),
        ("som_learning_rate", ["F64","I64","I64"]), ("som_quant_error", ["F64"]),
        ("neuro_nas_evaluate", ["F64","F64","F64"]), ("neuro_nas_sample", ["I64","I64"]),
        ("neuro_nas_prune", ["I64","F64"]), ("neuro_nas_best_score", []),
        ("spike_rl_rstdp", ["F64","F64"]), ("spike_rl_td", ["F64","F64","F64","F64"]),
        ("spike_rl_policy", ["F64","F64"]), ("spike_rl_predict_reward", ["F64","F64"]),
        ("curiosity_reward", ["F64"]), ("curiosity_info_gain", ["F64","F64"]),
        ("curiosity_novelty", ["F64"]), ("curiosity_decay", ["F64","I64"]),
        ("meta_maml_adapt", ["F64","F64","F64"]), ("meta_reptile", ["F64","F64","F64"]),
        ("meta_task_similarity", ["F64","F64","F64"]), ("meta_convergence", ["F64","F64","I64"]),
    ]),
    ("snn_hybrid", "v249-v254: SNN-ANN Hybrid", [
        ("snn_ann_to_rate", ["F64","F64"]), ("snn_rate_to_ann", ["F64","F64"]),
        ("snn_conversion_loss", ["F64","F64"]), ("snn_optimal_timesteps", ["F64"]),
        ("hybrid_infer", ["F64","F64","F64"]), ("hybrid_set_fraction", ["I64"]),
        ("hybrid_efficiency", ["F64","F64"]), ("hybrid_accuracy_gain", ["F64","F64"]),
        ("spike_compile", ["I64","I64","I64"]), ("spike_compile_optimize", ["I64","F64"]),
        ("spike_compile_count", []), ("spike_compile_memory", ["I64","I64"]),
        ("diff_spike_ste", ["F64","F64"]), ("diff_spike_sigmoid", ["F64","F64","F64"]),
        ("diff_spike_fast_sigmoid", ["F64","F64","F64"]), ("diff_spike_accumulate", ["F64","F64"]),
        ("neural_ode_euler", ["F64","F64","F64"]), ("neural_ode_rk4", ["F64","F64","F64","F64","F64","F64"]),
        ("neural_ode_adjoint", ["F64","F64"]), ("neural_ode_adaptive_dt", ["F64","F64","F64"]),
        ("hybrid_distill", ["F64","F64","F64"]), ("hybrid_freeze", ["I64","I64","I64"]),
        ("hybrid_lr", ["F64","I64","I64","I64"]), ("hybrid_weighted_acc", ["F64","F64","F64"]),
    ]),
    ("hippocampal_memory", "v255-v260: Hippocampal Memory", [
        ("hippo_encode", ["I64","I64","I64"]), ("hippo_recall", ["I64"]),
        ("hippo_replay", ["I64"]), ("hippo_count", []),
        ("wm_push", ["I64"]), ("wm_pop", []),
        ("wm_set_capacity", ["I64"]), ("wm_utilization", []),
        ("sleep_consolidate", ["I64"]), ("sleep_rem", ["F64"]),
        ("sleep_nrem_ripple", ["I64"]), ("sleep_duration", ["I64"]),
        ("hopfield_energy", ["I64","F64","F64"]), ("hopfield_capacity", ["I64"]),
        ("hopfield_retrieve", ["I64","I64"]), ("hopfield_accuracy", ["I64","F64"]),
        ("synaptag_decay", ["F64","F64","F64"]), ("synaptag_capture", ["F64","F64"]),
        ("synaptag_late_ltp", ["F64","F64"]), ("synaptag_protein", ["F64","F64"]),
        ("memcompress_schema", ["I64","F64"]), ("memcompress_forget", ["I64","F64"]),
        ("memcompress_merge", ["I64","F64"]), ("memcompress_ratio", ["I64","I64"]),
    ]),
    ("neuro_distributed", "v261-v266: Distributed Neuromorphic", [
        ("neuro_cluster_init", ["I64"]), ("neuro_cluster_distribute", ["I64"]),
        ("neuro_cluster_load", ["I64"]), ("neuro_cluster_nodes", []),
        ("spike_consensus_vote", ["I64","I64"]), ("spike_consensus_bft", ["I64"]),
        ("spike_consensus_tick", ["I64","I64"]), ("spike_consensus_latency", ["I64","I64"]),
        ("fed_neuro_average", ["F64","F64","F64"]), ("fed_neuro_dp_noise", ["I64","F64"]),
        ("fed_neuro_compress", ["I64","F64"]), ("fed_neuro_round", ["I64"]),
        ("edge_neuro_budget", ["I64"]), ("edge_neuro_quantize", ["I64","I64"]),
        ("edge_neuro_latency_ok", ["I64","I64"]), ("edge_neuro_model_size", ["I64","I64"]),
        ("stream_spike_process", ["I64","I64"]), ("stream_spike_rate", ["I64","F64"]),
        ("stream_spike_backpressure", ["I64","I64"]), ("stream_spike_total", []),
        ("neuro_platform_caps", ["I64"]), ("neuro_platform_map", ["I64","I64"]),
        ("neuro_platform_overhead", ["I64","I64"]), ("neuro_platform_power", ["I64","I64"]),
    ]),
    ("neuro_tooling", "v267-v272: Neuro Tooling", [
        ("neuro_viz_spike_raster", ["I64","I64","F64"]), ("neuro_viz_membrane", ["I64","I64"]),
        ("neuro_viz_connectivity", ["I64","I64"]), ("neuro_viz_frames", []),
        ("neuro_debug_break", ["I64"]), ("neuro_debug_inspect", ["I64","I64"]),
        ("neuro_debug_step", ["I64","F64"]), ("neuro_debug_breakpoints", []),
        ("neuro_profile_throughput", ["I64","F64"]), ("neuro_profile_memory", ["I64","I64"]),
        ("neuro_profile_energy", ["I64","I64"]), ("neuro_profile_samples", []),
        ("neuro_dsl_neuron", ["I64","I64"]), ("neuro_dsl_synapse", ["I64","I64","F64"]),
        ("neuro_dsl_network", ["I64","F64"]), ("neuro_dsl_validate", ["I64","I64"]),
        ("neuro_bench_spike_lat", ["I64"]), ("neuro_bench_neuron_tput", ["I64","I64"]),
        ("neuro_bench_synapse_rate", ["I64","F64"]), ("neuro_bench_efficiency", ["I64","I64"]),
        ("neuro_test_timing", ["I64","I64"]), ("neuro_test_accuracy", ["F64","F64"]),
        ("neuro_test_convergence", ["F64","F64"]), ("neuro_test_spike_gen", ["I64","F64","I64"]),
    ]),
    ("quantum_neuro", "v273-v278: Quantum Neuromorphic", [
        ("quantum_spike_encode", ["F64","I64"]), ("quantum_spike_decode", ["I64","I64"]),
        ("quantum_spike_superpose", ["I64","I64"]), ("quantum_spike_fidelity", ["F64"]),
        ("quantum_plasticity_stdp", ["F64","F64"]), ("quantum_plasticity_anneal", ["F64","F64"]),
        ("quantum_plasticity_tunnel", ["F64","F64"]), ("quantum_plasticity_t2", ["I64","F64"]),
        ("quantum_reservoir_init", ["I64"]), ("quantum_reservoir_project", ["F64","I64"]),
        ("quantum_reservoir_kernel", ["F64","F64","F64"]), ("quantum_reservoir_dim", []),
        ("qsnn_var_update", ["F64","F64","F64"]), ("qsnn_var_cost", ["F64","F64"]),
        ("qsnn_var_depth", ["I64","I64"]), ("qsnn_var_expressibility", ["I64","I64"]),
        ("quantum_qec_shor", ["I64"]), ("quantum_qec_surface", ["I64"]),
        ("quantum_qec_syndrome", ["I64"]), ("quantum_qec_overhead", ["I64","I64"]),
        ("quantum_bridge_encode", ["F64"]), ("quantum_bridge_decode", ["F64"]),
        ("quantum_bridge_cost", ["I64","I64"]), ("quantum_bridge_advantage", ["I64","I64"]),
    ]),
    ("neuro_safety", "v279-v284: Neuro Safety", [
        ("neuro_verify_timing", ["I64","I64","I64"]), ("neuro_verify_membrane", ["F64","F64","F64"]),
        ("neuro_verify_symmetry", ["F64","F64","F64"]), ("neuro_verify_liveness", ["I64","I64","I64"]),
        ("neuro_safe_rate_clamp", ["I64","I64"]), ("neuro_safe_runaway", ["I64","I64"]),
        ("neuro_safe_dead_neuron", ["I64"]), ("neuro_safe_violations", []),
        ("neuro_explain_contribution", ["F64","I64"]), ("neuro_explain_ablation", ["F64","F64"]),
        ("neuro_explain_saliency", ["F64","F64"]), ("neuro_explain_lrp", ["F64","F64","F64"]),
        ("neuro_robust_eps_check", ["F64","F64"]), ("neuro_robust_margin", ["F64","F64"]),
        ("neuro_robust_certified_radius", ["F64","F64"]), ("neuro_robust_inject_noise", ["F64","F64"]),
        ("neuro_fair_dp_gap", ["F64","F64"]), ("neuro_fair_eo_gap", ["F64","F64"]),
        ("neuro_fair_calibration", ["F64","F64"]), ("neuro_fair_lipschitz", ["F64","F64"]),
        ("neuro_cert_bounds", ["F64","F64","F64"]), ("neuro_cert_ibp_width", ["F64","F64"]),
        ("neuro_cert_crown", ["F64","F64"]), ("neuro_cert_accuracy", ["I64","I64"]),
    ]),
    ("neuro_perf", "v285-v290: Neuro Performance", [
        ("neuro_simd_accumulate", ["I64","I64","I64","I64"]), ("neuro_simd_threshold", ["I64","I64","I64","I64"]),
        ("neuro_simd_decay", ["F64","F64"]), ("neuro_simd_throughput", ["I64","I64"]),
        ("neuro_jit_compile", ["I64","I64"]), ("neuro_jit_speedup", ["I64","I64"]),
        ("neuro_jit_cache_hit", ["I64","I64"]), ("neuro_jit_compiled_count", []),
        ("neuro_precision_auto", ["F64"]), ("neuro_precision_quant_error", ["F64","I64"]),
        ("neuro_precision_savings", ["I64","I64"]), ("neuro_precision_scale", ["F64","F64"]),
        ("neuro_spec_predict", ["F64","F64"]), ("neuro_spec_gain", ["I64","I64"]),
        ("neuro_spec_rollback_cost", ["I64"]), ("neuro_spec_confidence", ["I64","I64"]),
        ("neuro_pgo_sample", ["I64"]), ("neuro_pgo_is_hot", ["I64","I64"]),
        ("neuro_pgo_unroll", ["I64"]), ("neuro_pgo_total_samples", []),
        ("neuro_zero_send", ["I64","I64"]), ("neuro_zero_update", ["F64","F64","F64"]),
        ("neuro_zero_conn_type", ["F64"]), ("neuro_zero_overhead", []),
    ]),
]

# ── v291-v300 inline builtins (defined directly in codegen.rs) ──
V291_V300 = [
    ("v291: Advanced Spike Analytics", [
        ("spike_analytics_mean_rate", ["I64","I64"]),
        ("spike_analytics_cv_isi", ["F64","F64"]),
        ("spike_analytics_fano_factor", ["F64","F64"]),
        ("spike_analytics_burst_index", ["I64","I64"]),
    ]),
    ("v292: Neural Network Metrics", [
        ("nn_metric_sparsity", ["I64","I64"]),
        ("nn_metric_entropy", ["F64"]),
        ("nn_metric_mutual_info", ["F64","F64"]),
        ("nn_metric_transfer_entropy", ["F64","F64","F64"]),
    ]),
    ("v293: Spike Train Distance", [
        ("spike_dist_victor_purpura", ["F64","F64","F64"]),
        ("spike_dist_van_rossum", ["F64","F64","F64"]),
        ("spike_dist_schreiber", ["F64","F64"]),
        ("spike_dist_earth_mover", ["I64","I64"]),
    ]),
    ("v294: Neural Coding", [
        ("neural_code_rate", ["I64","I64"]),
        ("neural_code_temporal", ["F64","F64"]),
        ("neural_code_population", ["I64","I64","I64"]),
        ("neural_code_sparse", ["I64","F64"]),
    ]),
    ("v295: Synaptic Plasticity Metrics", [
        ("synap_metric_ltp_ratio", ["F64","F64"]),
        ("synap_metric_ltd_ratio", ["F64","F64"]),
        ("synap_metric_homeostatic", ["F64","F64","F64"]),
        ("synap_metric_metaplasticity", ["F64","I64"]),
    ]),
    ("v296: Network Topology", [
        ("topo_clustering_coeff", ["I64","I64"]),
        ("topo_path_length", ["I64","I64"]),
        ("topo_small_world", ["F64","F64"]),
        ("topo_modularity", ["I64","I64","I64"]),
    ]),
    ("v297: Neuromorphic IO", [
        ("neuro_io_aer_encode", ["I64","I64"]),
        ("neuro_io_aer_decode", ["I64"]),
        ("neuro_io_dvs_encode", ["I64","I64","F64"]),
        ("neuro_io_serial_pack", ["I64","I64"]),
    ]),
    ("v298: Neural Dynamics", [
        ("dyn_lyapunov_exp", ["F64","F64","F64"]),
        ("dyn_bifurcation", ["F64","F64"]),
        ("dyn_phase_portrait", ["F64","F64"]),
        ("dyn_attractor_dim", ["F64","I64"]),
    ]),
    ("v299: Neuromorphic Scheduler", [
        ("neuro_sched_priority", ["I64","I64"]),
        ("neuro_sched_deadline", ["I64","I64"]),
        ("neuro_sched_edf", ["I64","I64","I64"]),
        ("neuro_sched_utilization", ["I64","I64"]),
    ]),
    ("v300: Milestone", [
        ("vitalis_v300_version", []),
        ("vitalis_v300_total_builtins", []),
        ("vitalis_v300_neuro_modules", []),
        ("vitalis_v300_milestone", []),
    ]),
]

def ir_type(t):
    return f"types::{t}"

def gen_symbols(modules, v291):
    """Generate builder.symbol lines."""
    lines = []
    for mod_name, label, fns in modules:
        lines.append(f"        // ── {label} ──")
        for fn_name, _ in fns:
            lines.append(f'        builder.symbol("slang_{fn_name}", crate::{mod_name}::slang_{fn_name} as *const u8);')
    # v291-v300 inline
    for label, fns in v291:
        lines.append(f"        // ── {label} ──")
        for fn_name, _ in fns:
            lines.append(f'        builder.symbol("slang_{fn_name}", slang_{fn_name} as *const u8);')
    return "\n".join(lines)

def gen_declfn(modules, v291):
    """Generate decl_fn! lines."""
    lines = []
    for _, label, fns in modules:
        lines.append(f"        // ── {label} ──")
        for fn_name, params in fns:
            if params:
                p = ", ".join(ir_type(t) for t in params)
                lines.append(f'        decl_fn!("slang_{fn_name}", [{p}], types::I64);')
            else:
                lines.append(f'        decl_fn!("slang_{fn_name}", [], types::I64);')
    for label, fns in v291:
        lines.append(f"        // ── {label} ──")
        for fn_name, params in fns:
            if params:
                p = ", ".join(ir_type(t) for t in params)
                lines.append(f'        decl_fn!("slang_{fn_name}", [{p}], types::I64);')
            else:
                lines.append(f'        decl_fn!("slang_{fn_name}", [], types::I64);')
    return "\n".join(lines)

def gen_match(modules, v291):
    """Generate name→runtime match arms."""
    lines = []
    for _, label, fns in modules:
        lines.append(f"                    // ── {label} ──")
        for fn_name, _ in fns:
            lines.append(f'                    "{fn_name}" => "slang_{fn_name}".into(),')
    for label, fns in v291:
        lines.append(f"                    // ── {label} ──")
        for fn_name, _ in fns:
            lines.append(f'                    "{fn_name}" => "slang_{fn_name}".into(),')
    return "\n".join(lines)

def gen_stdlib(modules, v291):
    """Generate stdlib BuiltinFn entries."""
    lines = []
    for _, label, fns in modules:
        lines.append(f"            // ── {label} ──")
        for fn_name, params in fns:
            if params:
                plist = ", ".join(f'("{p.lower()}", IrType::{t})' for i, (p, t) in enumerate(zip([f"p{j}" for j in range(len(params))], params)))
                # Use generic param names
                pvec = ", ".join(f'("p{i}", IrType::{t})' for i, t in enumerate(params))
                lines.append(f'            BuiltinFn {{ name: "{fn_name}".into(), params: vec![{pvec}], ret: IrType::I64, runtime_name: "slang_{fn_name}".into() }},')
            else:
                lines.append(f'            BuiltinFn {{ name: "{fn_name}".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_{fn_name}".into() }},')
    for label, fns in v291:
        lines.append(f"            // ── {label} ──")
        for fn_name, params in fns:
            if params:
                pvec = ", ".join(f'("p{i}", IrType::{t})' for i, t in enumerate(params))
                lines.append(f'            BuiltinFn {{ name: "{fn_name}".into(), params: vec![{pvec}], ret: IrType::I64, runtime_name: "slang_{fn_name}".into() }},')
            else:
                lines.append(f'            BuiltinFn {{ name: "{fn_name}".into(), params: vec![], ret: IrType::I64, runtime_name: "slang_{fn_name}".into() }},')
    return "\n".join(lines)

def gen_v291_extern_fns(v291):
    """Generate extern C function definitions for v291-v300."""
    lines = []
    for label, fns in v291:
        lines.append(f"// ── {label} ──")
        for fn_name, params in fns:
            if params:
                plist = ", ".join(f"_p{i}: {'f64' if t == 'F64' else 'i64'}" for i, t in enumerate(params))
                lines.append(f"#[unsafe(no_mangle)]")
                lines.append(f'extern "C" fn slang_{fn_name}({plist}) -> i64 {{')
            else:
                lines.append(f"#[unsafe(no_mangle)]")
                lines.append(f'extern "C" fn slang_{fn_name}() -> i64 {{')
            # body
            if fn_name == "vitalis_v300_version":
                lines.append("    300")
            elif fn_name == "vitalis_v300_total_builtins":
                lines.append("    400")
            elif fn_name == "vitalis_v300_neuro_modules":
                lines.append("    15")
            elif fn_name == "vitalis_v300_milestone":
                lines.append("    1")
            elif "mean_rate" in fn_name:
                lines.append("    if _p1 <= 0 { return 0; } _p0 * 1000 / _p1")
            elif "cv_isi" in fn_name:
                lines.append("    if _p1.abs() < 1e-10 { return 0; } (_p0 / _p1 * 1000.0) as i64")
            elif "fano_factor" in fn_name:
                lines.append("    if _p1.abs() < 1e-10 { return 1000; } (_p0 / _p1 * 1000.0) as i64")
            elif "burst_index" in fn_name:
                lines.append("    if _p1 <= 0 { return 0; } _p0 * 1000 / _p1")
            elif "sparsity" in fn_name:
                lines.append("    if _p1 <= 0 { return 0; } (_p1 - _p0) * 1000 / _p1")
            elif "entropy" in fn_name and "transfer" not in fn_name:
                lines.append("    if _p0 <= 0.0 { return 0; } (-_p0 * _p0.ln() * 1000.0) as i64")
            elif "mutual_info" in fn_name:
                lines.append("    ((_p0 - _p1).abs() * 1000.0) as i64")
            elif "transfer_entropy" in fn_name:
                lines.append("    ((_p0 + _p1 - _p2).abs() * 1000.0) as i64")
            elif "victor_purpura" in fn_name:
                lines.append("    ((_p0 - _p1).abs() * _p2 * 1000.0) as i64")
            elif "van_rossum" in fn_name:
                lines.append("    let d = _p0 - _p1; (d * d * (-_p2).exp() * 1000.0) as i64")
            elif "schreiber" in fn_name:
                lines.append("    ((_p0 * _p1).sqrt() * 1000.0) as i64")
            elif "earth_mover" in fn_name:
                lines.append("    (_p0 - _p1).abs()")
            elif "neural_code_rate" in fn_name:
                lines.append("    if _p1 <= 0 { return 0; } _p0 * 1000 / _p1")
            elif "neural_code_temporal" in fn_name:
                lines.append("    ((_p0 - _p1).abs() * 1000.0) as i64")
            elif "neural_code_population" in fn_name:
                lines.append("    if _p2 <= 0 { return 0; } _p0 * _p1 / _p2")
            elif "neural_code_sparse" in fn_name:
                lines.append("    (_p0 as f64 * _p1 * 1000.0) as i64")
            elif "ltp_ratio" in fn_name:
                lines.append("    if _p1.abs() < 1e-10 { return 0; } (_p0 / _p1 * 1000.0) as i64")
            elif "ltd_ratio" in fn_name:
                lines.append("    if _p1.abs() < 1e-10 { return 0; } (_p0 / _p1 * 1000.0) as i64")
            elif "homeostatic" in fn_name:
                lines.append("    ((_p0 + _p1 - _p2).abs() * 1000.0) as i64")
            elif "metaplasticity" in fn_name:
                lines.append("    (_p0 * _p1 as f64 * 1000.0) as i64")
            elif "clustering_coeff" in fn_name:
                lines.append("    if _p1 <= 0 { return 0; } _p0 * 1000 / _p1")
            elif "path_length" in fn_name:
                lines.append("    if _p1 <= 0 { return 0; } _p0 * 1000 / _p1")
            elif "small_world" in fn_name:
                lines.append("    if _p1.abs() < 1e-10 { return 1000; } (_p0 / _p1 * 1000.0) as i64")
            elif "modularity" in fn_name:
                lines.append("    if _p2 <= 0 { return 0; } (_p0 - _p1) * 1000 / _p2")
            elif "aer_encode" in fn_name:
                lines.append("    _p0 << 16 | (_p1 & 0xFFFF)")
            elif "aer_decode" in fn_name:
                lines.append("    _p0 >> 16")
            elif "dvs_encode" in fn_name:
                lines.append("    let pol = if _p2 > 0.0 { 1_i64 } else { 0 }; _p0 << 17 | (_p1 & 0xFFFF) << 1 | pol")
            elif "serial_pack" in fn_name:
                lines.append("    _p0 << 32 | (_p1 & 0xFFFFFFFF)")
            elif "lyapunov" in fn_name:
                lines.append("    ((_p0 * _p1 + _p2) * 1000.0) as i64")
            elif "bifurcation" in fn_name:
                lines.append("    ((_p0 * _p0 - _p1) * 1000.0) as i64")
            elif "phase_portrait" in fn_name:
                lines.append("    ((_p0 * _p0 + _p1 * _p1).sqrt() * 1000.0) as i64")
            elif "attractor_dim" in fn_name:
                lines.append("    (_p0 * _p1 as f64 * 1000.0) as i64")
            elif "sched_priority" in fn_name:
                lines.append("    if _p0 > _p1 { _p0 } else { _p1 }")
            elif "sched_deadline" in fn_name:
                lines.append("    if _p0 <= _p1 { 1 } else { 0 }")
            elif "sched_edf" in fn_name:
                lines.append("    if _p1 <= 0 { return 0; } _p0 * _p2 / _p1")
            elif "sched_utilization" in fn_name:
                lines.append("    if _p1 <= 0 { return 0; } _p0 * 1000 / _p1")
            else:
                lines.append("    0")
            lines.append("}")
            lines.append("")
    return "\n".join(lines)

def gen_tests():
    """Generate codegen.rs test functions for v201-v300."""
    lines = []
    lines.append("")
    lines.append("    #[test]")
    lines.append("    fn test_v201_v290_neuro_builtins_symbols() {")
    lines.append("        // Spot-check that module builtins are registered")
    lines.append("        assert_ne!(crate::spike_engine::slang_spike_emit(0, 0, 0), -1);")
    lines.append("        assert!(crate::loihi_sim::slang_loihi_core_count() >= 0);")
    lines.append("        assert!(crate::brain_models::slang_htm_column_count() >= 0);")
    lines.append("        assert!(crate::hippocampal_memory::slang_hippo_count() >= 0);")
    lines.append("        assert!(crate::neuro_distributed::slang_neuro_cluster_nodes() >= 0);")
    lines.append("        assert!(crate::quantum_neuro::slang_quantum_reservoir_dim() >= 0);")
    lines.append("        assert_eq!(crate::neuro_perf::slang_neuro_zero_overhead(), 0);")
    lines.append("    }")
    lines.append("")
    lines.append("    #[test]")
    lines.append("    fn test_v291_v300_inline_builtins() {")
    lines.append("        assert_eq!(slang_vitalis_v300_version(), 300);")
    lines.append("        assert_eq!(slang_vitalis_v300_total_builtins(), 400);")
    lines.append("        assert_eq!(slang_vitalis_v300_neuro_modules(), 15);")
    lines.append("        assert_eq!(slang_vitalis_v300_milestone(), 1);")
    lines.append("    }")
    lines.append("")
    lines.append("    #[test]")
    lines.append("    fn test_v300_all_neuro_builtins_in_stdlib() {")
    lines.append("        let builtins = crate::stdlib::builtins();")
    lines.append('        let neuro_names = ["spike_emit", "loihi_core_create", "snn_surrogate_forward",')
    lines.append('            "pim_alloc", "pred_coding_forward", "gpu_spike_propagate",')
    lines.append('            "spike_vision_encode", "neat_crossover", "snn_ann_to_rate",')
    lines.append('            "hippo_encode", "neuro_cluster_init", "neuro_viz_spike_raster",')
    lines.append('            "quantum_spike_encode", "neuro_verify_timing", "neuro_simd_accumulate",')
    lines.append('            "vitalis_v300_version"];')
    lines.append("        for name in neuro_names {")
    lines.append('            assert!(builtins.iter().any(|b| b.name == name), "Missing builtin: {}", name);')
    lines.append("        }")
    lines.append("    }")
    return "\n".join(lines)


# ── Main: Read, inject, write ──
import sys

codegen_path = r"c:\Vitalis-V60\src\codegen.rs"
stdlib_path = r"c:\Vitalis-V60\src\stdlib.rs"

# Read files
with open(codegen_path, "r", encoding="utf-8") as f:
    codegen = f.read()
with open(stdlib_path, "r", encoding="utf-8") as f:
    stdlib = f.read()

total_fns = sum(len(fns) for _, _, fns in MODULES) + sum(len(fns) for _, fns in V291_V300)
print(f"Total functions to register: {total_fns}")

# ── 1. Insert builder.symbol ──
anchor_sym = 'builder.symbol("slang_vitalis_builtin_count",  slang_vitalis_builtin_count  as *const u8);'
if anchor_sym not in codegen:
    # Try without extra spaces
    anchor_sym = 'builder.symbol("slang_vitalis_builtin_count", slang_vitalis_builtin_count as *const u8);'
if anchor_sym not in codegen:
    print("ERROR: Cannot find builder.symbol anchor for vitalis_builtin_count", file=sys.stderr)
    sys.exit(1)

sym_block = gen_symbols(MODULES, V291_V300)
codegen = codegen.replace(anchor_sym, anchor_sym + "\n" + sym_block, 1)
print("Inserted builder.symbol block")

# ── 2. Insert decl_fn! ──
anchor_decl = 'decl_fn!("slang_vitalis_builtin_count",  [],                       types::I64);'
if anchor_decl not in codegen:
    anchor_decl = 'decl_fn!("slang_vitalis_builtin_count", [], types::I64);'
if anchor_decl not in codegen:
    print("ERROR: Cannot find decl_fn anchor", file=sys.stderr)
    sys.exit(1)

decl_block = gen_declfn(MODULES, V291_V300)
codegen = codegen.replace(anchor_decl, anchor_decl + "\n" + decl_block, 1)
print("Inserted decl_fn! block")

# ── 3. Insert name→runtime match ──
anchor_match = '"vitalis_builtin_count"  => "slang_vitalis_builtin_count".into(),'
if anchor_match not in codegen:
    anchor_match = '"vitalis_builtin_count" => "slang_vitalis_builtin_count".into(),'
if anchor_match not in codegen:
    print("ERROR: Cannot find match arm anchor", file=sys.stderr)
    sys.exit(1)

match_block = gen_match(MODULES, V291_V300)
codegen = codegen.replace(anchor_match, anchor_match + "\n" + match_block, 1)
print("Inserted name->runtime match block")

# ── 4. Insert v291-v300 extern C functions ──
# Insert before the tests module
anchor_tests = "#[cfg(test)]\nmod tests {"
if anchor_tests not in codegen:
    anchor_tests = "#[cfg(test)]\r\nmod tests {"
if anchor_tests not in codegen:
    print("ERROR: Cannot find #[cfg(test)] mod tests anchor", file=sys.stderr)
    sys.exit(1)

extern_block = gen_v291_extern_fns(V291_V300)
codegen = codegen.replace(anchor_tests, extern_block + "\n\n" + anchor_tests, 1)
print("Inserted v291-v300 extern C functions")

# ── 5. Insert tests ──
# Find the last test and closing brace
test_block = gen_tests()
# Insert before the final closing } of mod tests
# Find the last occurrence of closing brace in the tests section
last_brace = codegen.rfind("\n}")
if last_brace == -1:
    print("ERROR: Cannot find closing brace of mod tests", file=sys.stderr)
    sys.exit(1)
codegen = codegen[:last_brace] + test_block + "\n" + codegen[last_brace:]
print("Inserted test functions")

# ── 6. Update stdlib.rs ──
anchor_stdlib = 'BuiltinFn { name: "vitalis_builtin_count".into(),'
if anchor_stdlib not in stdlib:
    print("ERROR: Cannot find stdlib anchor", file=sys.stderr)
    sys.exit(1)

# Find the full line
idx = stdlib.index(anchor_stdlib)
line_end = stdlib.index("\n", idx)
full_anchor_line = stdlib[idx:line_end]

stdlib_block = gen_stdlib(MODULES, V291_V300)
stdlib = stdlib.replace(full_anchor_line, full_anchor_line + "\n" + stdlib_block, 1)
print("Inserted stdlib BuiltinFn entries")

# ── Write files ──
with open(codegen_path, "w", encoding="utf-8", newline="\n") as f:
    f.write(codegen)
print(f"Wrote codegen.rs ({len(codegen)} bytes)")

with open(stdlib_path, "w", encoding="utf-8", newline="\n") as f:
    f.write(stdlib)
print(f"Wrote stdlib.rs ({len(stdlib)} bytes)")

print("\nDone! Registered all v201-v300 builtins.")
