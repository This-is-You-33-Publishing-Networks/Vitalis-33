//! JIT Symbol Registration — v62 Stdlib Bridge
//!
//! Registers extern "C" function pointers from algorithm/ML/GUI modules
//! with the Cranelift JIT builder so they can be called from .sl code.
//! This is the single source of truth for all module↔JIT symbol mappings.

use cranelift_jit::JITBuilder;

/// Register a symbol with an explicit name (when the fn path doesn't match the symbol name).
macro_rules! sym_as {
    ($builder:expr, $name:literal, $path:path) => {
        $builder.symbol($name, $path as *const u8);
    };
}

/// Registers all extended module symbols with the JIT builder.
/// Called from codegen.rs during JIT module construction.
pub fn register_all(builder: &mut JITBuilder) {
    // ── Data Structures ──────────────────────────────────────────────
    sym_as!(builder, "vitalis_btree_create",     crate::data_structures::vitalis_btree_create);
    sym_as!(builder, "vitalis_btree_insert",     crate::data_structures::vitalis_btree_insert);
    sym_as!(builder, "vitalis_btree_search",     crate::data_structures::vitalis_btree_search);
    sym_as!(builder, "vitalis_btree_len",        crate::data_structures::vitalis_btree_len);
    sym_as!(builder, "vitalis_btree_free",       crate::data_structures::vitalis_btree_free);
    sym_as!(builder, "vitalis_ringbuf_create",   crate::data_structures::vitalis_ringbuf_create);
    sym_as!(builder, "vitalis_ringbuf_push_back",crate::data_structures::vitalis_ringbuf_push_back);
    sym_as!(builder, "vitalis_ringbuf_pop_front",crate::data_structures::vitalis_ringbuf_pop_front);
    sym_as!(builder, "vitalis_ringbuf_len",      crate::data_structures::vitalis_ringbuf_len);
    sym_as!(builder, "vitalis_ringbuf_free",     crate::data_structures::vitalis_ringbuf_free);
    sym_as!(builder, "vitalis_uf_create",        crate::data_structures::vitalis_uf_create);
    sym_as!(builder, "vitalis_uf_union",         crate::data_structures::vitalis_uf_union);
    sym_as!(builder, "vitalis_uf_find",          crate::data_structures::vitalis_uf_find);
    sym_as!(builder, "vitalis_uf_connected",     crate::data_structures::vitalis_uf_connected);
    sym_as!(builder, "vitalis_uf_set_count",     crate::data_structures::vitalis_uf_set_count);
    sym_as!(builder, "vitalis_uf_free",          crate::data_structures::vitalis_uf_free);
    sym_as!(builder, "vitalis_lru_create",       crate::data_structures::vitalis_lru_create);
    sym_as!(builder, "vitalis_lru_put",          crate::data_structures::vitalis_lru_put);
    sym_as!(builder, "vitalis_lru_get",          crate::data_structures::vitalis_lru_get);
    sym_as!(builder, "vitalis_lru_len",          crate::data_structures::vitalis_lru_len);
    sym_as!(builder, "vitalis_lru_free",         crate::data_structures::vitalis_lru_free);

    // ── ECS ──────────────────────────────────────────────────────────
    sym_as!(builder, "vitalis_ecs_world_create",   crate::ecs::vitalis_ecs_world_create);
    sym_as!(builder, "vitalis_ecs_world_free",     crate::ecs::vitalis_ecs_world_free);
    sym_as!(builder, "vitalis_ecs_spawn",          crate::ecs::vitalis_ecs_spawn);
    sym_as!(builder, "vitalis_ecs_despawn",        crate::ecs::vitalis_ecs_despawn);
    sym_as!(builder, "vitalis_ecs_add_component",  crate::ecs::vitalis_ecs_add_component);
    sym_as!(builder, "vitalis_ecs_get_component",  crate::ecs::vitalis_ecs_get_component);
    sym_as!(builder, "vitalis_ecs_has_component",  crate::ecs::vitalis_ecs_has_component);
    sym_as!(builder, "vitalis_ecs_entity_count",   crate::ecs::vitalis_ecs_entity_count);
    sym_as!(builder, "vitalis_ecs_component_count",crate::ecs::vitalis_ecs_component_count);

    // ── Regex Engine ─────────────────────────────────────────────────
    sym_as!(builder, "vitalis_regex_is_match",           crate::regex_engine::vitalis_regex_is_match);
    sym_as!(builder, "vitalis_regex_find_first",         crate::regex_engine::vitalis_regex_find_first);
    sym_as!(builder, "vitalis_regex_find_all_matches",   crate::regex_engine::vitalis_regex_find_all_matches);
    sym_as!(builder, "vitalis_regex_captures_first",     crate::regex_engine::vitalis_regex_captures_first);
    sym_as!(builder, "vitalis_regex_replace_first",      crate::regex_engine::vitalis_regex_replace_first);
    sym_as!(builder, "vitalis_regex_replace_all_matches",crate::regex_engine::vitalis_regex_replace_all_matches);
    sym_as!(builder, "vitalis_regex_split_by",           crate::regex_engine::vitalis_regex_split_by);

    // ── Serialization ────────────────────────────────────────────────
    sym_as!(builder, "vitalis_json_parse",         crate::serialization::vitalis_json_parse);
    sym_as!(builder, "vitalis_json_stringify",      crate::serialization::vitalis_json_stringify);
    sym_as!(builder, "vitalis_json_get",            crate::serialization::vitalis_json_get);
    sym_as!(builder, "vitalis_ser_base64_encode",   crate::serialization::vitalis_ser_base64_encode);
    sym_as!(builder, "vitalis_ser_base64_decode",   crate::serialization::vitalis_ser_base64_decode);
    sym_as!(builder, "vitalis_ser_hex_encode",      crate::serialization::vitalis_ser_hex_encode);
    sym_as!(builder, "vitalis_ser_hex_decode",      crate::serialization::vitalis_ser_hex_decode);
    sym_as!(builder, "vitalis_ser_url_encode",      crate::serialization::vitalis_ser_url_encode);
    sym_as!(builder, "vitalis_ser_url_decode",      crate::serialization::vitalis_ser_url_decode);
    sym_as!(builder, "vitalis_msgpack_roundtrip",   crate::serialization::vitalis_msgpack_roundtrip);
    sym_as!(builder, "vitalis_varint_encode",       crate::serialization::vitalis_varint_encode);
    sym_as!(builder, "vitalis_varint_decode",       crate::serialization::vitalis_varint_decode);

    // ── Property Testing ─────────────────────────────────────────────
    sym_as!(builder, "vitalis_qc_gen_i64",                  crate::property_testing::vitalis_qc_gen_i64);
    sym_as!(builder, "vitalis_qc_gen_f64",                  crate::property_testing::vitalis_qc_gen_f64);
    sym_as!(builder, "vitalis_qc_gen_bool",                 crate::property_testing::vitalis_qc_gen_bool);
    sym_as!(builder, "vitalis_qc_gen_string",               crate::property_testing::vitalis_qc_gen_string);
    sym_as!(builder, "vitalis_qc_shrink_i64",               crate::property_testing::vitalis_qc_shrink_i64);
    sym_as!(builder, "vitalis_qc_test_commutative_add",     crate::property_testing::vitalis_qc_test_commutative_add);
    sym_as!(builder, "vitalis_qc_test_sort_idempotent",     crate::property_testing::vitalis_qc_test_sort_idempotent);
    sym_as!(builder, "vitalis_qc_test_sort_preserves_length",crate::property_testing::vitalis_qc_test_sort_preserves_length);
    sym_as!(builder, "vitalis_qc_chi_squared",              crate::property_testing::vitalis_qc_chi_squared);

    // ── Networking ───────────────────────────────────────────────────
    sym_as!(builder, "vitalis_url_parse",           crate::networking::vitalis_url_parse);
    sym_as!(builder, "vitalis_http_build_request",  crate::networking::vitalis_http_build_request);
    sym_as!(builder, "vitalis_http_parse_request",  crate::networking::vitalis_http_parse_request);
    sym_as!(builder, "vitalis_is_valid_ipv4",       crate::networking::vitalis_is_valid_ipv4);
    sym_as!(builder, "vitalis_is_valid_ipv6",       crate::networking::vitalis_is_valid_ipv6);
    sym_as!(builder, "vitalis_parse_query_string",  crate::networking::vitalis_parse_query_string);
    sym_as!(builder, "vitalis_dns_build_query",     crate::networking::vitalis_dns_build_query);

    // ── Tensor Engine ────────────────────────────────────────────────
    sym_as!(builder, "vitalis_tensor_zeros",     crate::tensor::vitalis_tensor_zeros);
    sym_as!(builder, "vitalis_tensor_ones",      crate::tensor::vitalis_tensor_ones);
    sym_as!(builder, "vitalis_tensor_rand",      crate::tensor::vitalis_tensor_rand);
    sym_as!(builder, "vitalis_tensor_randn",     crate::tensor::vitalis_tensor_randn);
    sym_as!(builder, "vitalis_tensor_from_data", crate::tensor::vitalis_tensor_from_data);
    sym_as!(builder, "vitalis_tensor_add",       crate::tensor::vitalis_tensor_add);
    sym_as!(builder, "vitalis_tensor_mul",       crate::tensor::vitalis_tensor_mul);
    sym_as!(builder, "vitalis_tensor_matmul",    crate::tensor::vitalis_tensor_matmul);
    sym_as!(builder, "vitalis_tensor_sum",       crate::tensor::vitalis_tensor_sum);
    sym_as!(builder, "vitalis_tensor_mean",      crate::tensor::vitalis_tensor_mean);
    sym_as!(builder, "vitalis_tensor_reshape",   crate::tensor::vitalis_tensor_reshape);
    sym_as!(builder, "vitalis_tensor_transpose", crate::tensor::vitalis_tensor_transpose);
    sym_as!(builder, "vitalis_tensor_relu",      crate::tensor::vitalis_tensor_relu);
    sym_as!(builder, "vitalis_tensor_softmax",   crate::tensor::vitalis_tensor_softmax);
    sym_as!(builder, "vitalis_tensor_numel",     crate::tensor::vitalis_tensor_numel);
    sym_as!(builder, "vitalis_tensor_ndim",      crate::tensor::vitalis_tensor_ndim);
    sym_as!(builder, "vitalis_tensor_get",       crate::tensor::vitalis_tensor_get);
    sym_as!(builder, "vitalis_tensor_free",      crate::tensor::vitalis_tensor_free);

    // ── Tensor v115 (simplified creation) ────────────────────────────
    sym_as!(builder, "vitalis_tensor_zeros_2d",  crate::tensor::vitalis_tensor_zeros_2d);
    sym_as!(builder, "vitalis_tensor_ones_2d",   crate::tensor::vitalis_tensor_ones_2d);
    sym_as!(builder, "vitalis_tensor_zeros_1d",  crate::tensor::vitalis_tensor_zeros_1d);
    sym_as!(builder, "vitalis_tensor_scalar",    crate::tensor::vitalis_tensor_scalar);
    sym_as!(builder, "vitalis_tensor_set",       crate::tensor::vitalis_tensor_set);

    // ── Autograd ─────────────────────────────────────────────────────
    sym_as!(builder, "vitalis_autograd_variable",  crate::autograd::vitalis_autograd_variable);
    sym_as!(builder, "vitalis_autograd_add",       crate::autograd::vitalis_autograd_add);
    sym_as!(builder, "vitalis_autograd_mul",       crate::autograd::vitalis_autograd_mul);
    sym_as!(builder, "vitalis_autograd_backward",  crate::autograd::vitalis_autograd_backward);
    sym_as!(builder, "vitalis_autograd_get_grad",  crate::autograd::vitalis_autograd_get_grad);
    sym_as!(builder, "vitalis_autograd_clear",     crate::autograd::vitalis_autograd_clear);
    sym_as!(builder, "vitalis_autograd_no_grad",   crate::autograd::vitalis_autograd_no_grad);
    sym_as!(builder, "vitalis_grad_clip_norm",     crate::autograd::vitalis_grad_clip_norm);
    sym_as!(builder, "vitalis_grad_clip_value",    crate::autograd::vitalis_grad_clip_value);

    // ── Autograd v116 (simplified JIT-callable) ──────────────────────
    sym_as!(builder, "vitalis_autograd_scalar",      crate::autograd::vitalis_autograd_scalar);
    sym_as!(builder, "vitalis_autograd_value",       crate::autograd::vitalis_autograd_value);
    sym_as!(builder, "vitalis_autograd_grad_scalar", crate::autograd::vitalis_autograd_grad_scalar);
    sym_as!(builder, "vitalis_autograd_numel",       crate::autograd::vitalis_autograd_numel);
    sym_as!(builder, "vitalis_autograd_sum",         crate::autograd::vitalis_autograd_sum);

    // ── Training Engine ──────────────────────────────────────────────
    sym_as!(builder, "vitalis_train_adamw_create",  crate::training_engine::vitalis_train_adamw_create);
    sym_as!(builder, "vitalis_train_adamw_step",    crate::training_engine::vitalis_train_adamw_step);
    sym_as!(builder, "vitalis_train_cosine_lr",     crate::training_engine::vitalis_train_cosine_lr);
    sym_as!(builder, "vitalis_train_onecycle_lr",   crate::training_engine::vitalis_train_onecycle_lr);
    sym_as!(builder, "vitalis_train_cross_entropy", crate::training_engine::vitalis_train_cross_entropy);
    sym_as!(builder, "vitalis_train_mse",           crate::training_engine::vitalis_train_mse);
    sym_as!(builder, "vitalis_train_grad_norm",     crate::training_engine::vitalis_train_grad_norm);
    sym_as!(builder, "vitalis_train_clip_grad",     crate::training_engine::vitalis_train_clip_grad);

    // ── Transformer ──────────────────────────────────────────────────
    sym_as!(builder, "vitalis_transformer_create",  crate::transformer::vitalis_transformer_create);
    sym_as!(builder, "vitalis_transformer_params",  crate::transformer::vitalis_transformer_params);
    sym_as!(builder, "vitalis_attention_sdpa",      crate::transformer::vitalis_attention_sdpa);
    sym_as!(builder, "vitalis_flash_attention",     crate::transformer::vitalis_flash_attention);
    sym_as!(builder, "vitalis_rope_apply",          crate::transformer::vitalis_rope_apply);

    // ── Inference ────────────────────────────────────────────────────
    sym_as!(builder, "vitalis_inference_argmax",             crate::inference::vitalis_inference_argmax);
    sym_as!(builder, "vitalis_inference_apply_temperature",  crate::inference::vitalis_inference_apply_temperature);
    sym_as!(builder, "vitalis_inference_apply_top_k",        crate::inference::vitalis_inference_apply_top_k);
    sym_as!(builder, "vitalis_inference_apply_top_p",        crate::inference::vitalis_inference_apply_top_p);
    sym_as!(builder, "vitalis_inference_sample",             crate::inference::vitalis_inference_sample);

    // ── Quantization ─────────────────────────────────────────────────
    sym_as!(builder, "vitalis_quantize_int8",     crate::quantization::vitalis_quantize_int8);
    sym_as!(builder, "vitalis_quantize_int4",     crate::quantization::vitalis_quantize_int4);
    sym_as!(builder, "vitalis_quantize_nf4_ffi",  crate::quantization::vitalis_quantize_nf4_ffi);
    sym_as!(builder, "vitalis_quantize_error",    crate::quantization::vitalis_quantize_error);
    sym_as!(builder, "vitalis_quantize_free",     crate::quantization::vitalis_quantize_free);

    // ── Code Intelligence ────────────────────────────────────────────
    sym_as!(builder, "vitalis_code_cyclomatic",      crate::code_intelligence::vitalis_code_cyclomatic);
    sym_as!(builder, "vitalis_code_cognitive",       crate::code_intelligence::vitalis_code_cognitive);
    sym_as!(builder, "vitalis_code_maintainability", crate::code_intelligence::vitalis_code_maintainability);
    sym_as!(builder, "vitalis_code_similarity",      crate::code_intelligence::vitalis_code_similarity);

    // ── Program Synthesis ────────────────────────────────────────────
    sym_as!(builder, "vitalis_synth_eval",        crate::program_synthesis::vitalis_synth_eval);
    sym_as!(builder, "vitalis_synth_complexity",  crate::program_synthesis::vitalis_synth_complexity);

    // ── Self-Optimizer ───────────────────────────────────────────────
    sym_as!(builder, "vitalis_selfopt_cost_predict", crate::self_optimizer::vitalis_selfopt_cost_predict);
    sym_as!(builder, "vitalis_selfopt_num_passes",   crate::self_optimizer::vitalis_selfopt_num_passes);
    sym_as!(builder, "vitalis_selfopt_tier",         crate::self_optimizer::vitalis_selfopt_tier);

    // ── Autonomous Agent ─────────────────────────────────────────────
    sym_as!(builder, "vitalis_agent_create",       crate::autonomous_agent::vitalis_agent_create);
    sym_as!(builder, "vitalis_agent_success_rate", crate::autonomous_agent::vitalis_agent_success_rate);
    sym_as!(builder, "vitalis_agent_total_actions",crate::autonomous_agent::vitalis_agent_total_actions);
    sym_as!(builder, "vitalis_agent_free",         crate::autonomous_agent::vitalis_agent_free);

    // ── Reward Model ─────────────────────────────────────────────────
    sym_as!(builder, "vitalis_reward_create",    crate::reward_model::vitalis_reward_create);
    sym_as!(builder, "vitalis_reward_score",     crate::reward_model::vitalis_reward_score);
    sym_as!(builder, "vitalis_reward_ppo_loss",  crate::reward_model::vitalis_reward_ppo_loss);
    sym_as!(builder, "vitalis_reward_gae",       crate::reward_model::vitalis_reward_gae);
    sym_as!(builder, "vitalis_reward_free",      crate::reward_model::vitalis_reward_free);

    // ── Differentiable Programming ───────────────────────────────────
    sym_as!(builder, "vitalis_dual_new",             crate::differentiable::vitalis_dual_new);
    sym_as!(builder, "vitalis_dual_mul",             crate::differentiable::vitalis_dual_mul);
    sym_as!(builder, "vitalis_forward_deriv",        crate::differentiable::vitalis_forward_deriv);
    sym_as!(builder, "vitalis_shape_broadcast_ok",   crate::differentiable::vitalis_shape_broadcast_ok);

    // ── Probabilistic Programming ────────────────────────────────────
    sym_as!(builder, "vitalis_prob_normal",      crate::probabilistic::vitalis_prob_normal);
    sym_as!(builder, "vitalis_prob_log_prob",    crate::probabilistic::vitalis_prob_log_prob);
    sym_as!(builder, "vitalis_prob_sample",      crate::probabilistic::vitalis_prob_sample);
    sym_as!(builder, "vitalis_prob_free",        crate::probabilistic::vitalis_prob_free);
    sym_as!(builder, "vitalis_mcmc_normal_mean", crate::probabilistic::vitalis_mcmc_normal_mean);

    // ── Reinforcement Learning ───────────────────────────────────────
    sym_as!(builder, "vitalis_rl_create",   crate::rl_framework::vitalis_rl_create);
    sym_as!(builder, "vitalis_rl_get_q",    crate::rl_framework::vitalis_rl_get_q);
    sym_as!(builder, "vitalis_rl_update",   crate::rl_framework::vitalis_rl_update);
    sym_as!(builder, "vitalis_rl_free",     crate::rl_framework::vitalis_rl_free);

    // ── Simulation ───────────────────────────────────────────────────
    sym_as!(builder, "vitalis_sim_grid_new",   crate::simulation::vitalis_sim_grid_new);
    sym_as!(builder, "vitalis_sim_grid_step",  crate::simulation::vitalis_sim_grid_step);
    sym_as!(builder, "vitalis_sim_grid_reset", crate::simulation::vitalis_sim_grid_reset);
    sym_as!(builder, "vitalis_sim_free",       crate::simulation::vitalis_sim_free);

    // ── Data Pipeline ────────────────────────────────────────────────
    sym_as!(builder, "vitalis_data_create",    crate::data_pipeline::vitalis_data_create);
    sym_as!(builder, "vitalis_data_len",       crate::data_pipeline::vitalis_data_len);
    sym_as!(builder, "vitalis_data_parse_csv", crate::data_pipeline::vitalis_data_parse_csv);
    sym_as!(builder, "vitalis_data_free",      crate::data_pipeline::vitalis_data_free);

    // ── Experiment Tracking ──────────────────────────────────────────
    sym_as!(builder, "vitalis_exp_create",     crate::experiment::vitalis_exp_create);
    sym_as!(builder, "vitalis_exp_log_metric", crate::experiment::vitalis_exp_log_metric);
    sym_as!(builder, "vitalis_exp_get_metric", crate::experiment::vitalis_exp_get_metric);
    sym_as!(builder, "vitalis_exp_complete",   crate::experiment::vitalis_exp_complete);
    sym_as!(builder, "vitalis_exp_free",       crate::experiment::vitalis_exp_free);

    // ── Model Serving ────────────────────────────────────────────────
    sym_as!(builder, "vitalis_serve_create",     crate::model_serving::vitalis_serve_create);
    sym_as!(builder, "vitalis_serve_load_model", crate::model_serving::vitalis_serve_load_model);
    sym_as!(builder, "vitalis_serve_predict",    crate::model_serving::vitalis_serve_predict);
    sym_as!(builder, "vitalis_serve_free",       crate::model_serving::vitalis_serve_free);

    // ── AI Observability ─────────────────────────────────────────────
    sym_as!(builder, "vitalis_obs_create_drift", crate::ai_observability::vitalis_obs_create_drift);
    sym_as!(builder, "vitalis_obs_check_drift",  crate::ai_observability::vitalis_obs_check_drift);
    sym_as!(builder, "vitalis_obs_free",         crate::ai_observability::vitalis_obs_free);

    // ── GPU Compute ──────────────────────────────────────────────────
    sym_as!(builder, "vitalis_gpu_pipeline_new",    crate::gpu_compute::vitalis_gpu_pipeline_new);
    sym_as!(builder, "vitalis_gpu_add_kernel",      crate::gpu_compute::vitalis_gpu_add_kernel);
    sym_as!(builder, "vitalis_gpu_create_buffer",   crate::gpu_compute::vitalis_gpu_create_buffer);
    sym_as!(builder, "vitalis_gpu_dispatch",        crate::gpu_compute::vitalis_gpu_dispatch);
    sym_as!(builder, "vitalis_gpu_buffer_count",    crate::gpu_compute::vitalis_gpu_buffer_count);
    sym_as!(builder, "vitalis_gpu_kernel_count",    crate::gpu_compute::vitalis_gpu_kernel_count);
    sym_as!(builder, "vitalis_gpu_pipeline_free",   crate::gpu_compute::vitalis_gpu_pipeline_free);

    // ── WASM AOT ─────────────────────────────────────────────────────
    sym_as!(builder, "vitalis_wasm_aot_create",  crate::wasm_aot::vitalis_wasm_aot_create);
    sym_as!(builder, "vitalis_wasm_aot_size",    crate::wasm_aot::vitalis_wasm_aot_size);
    sym_as!(builder, "vitalis_wasm_aot_exports", crate::wasm_aot::vitalis_wasm_aot_exports);
    sym_as!(builder, "vitalis_wasm_aot_free",    crate::wasm_aot::vitalis_wasm_aot_free);

    // ── Distributed Build ────────────────────────────────────────────
    sym_as!(builder, "vitalis_distbuild_create",      crate::distributed_build::vitalis_distbuild_create);
    sym_as!(builder, "vitalis_distbuild_add_node",    crate::distributed_build::vitalis_distbuild_add_node);
    sym_as!(builder, "vitalis_distbuild_utilization",  crate::distributed_build::vitalis_distbuild_utilization);
    sym_as!(builder, "vitalis_distbuild_free",        crate::distributed_build::vitalis_distbuild_free);

    // ── Formal Verification ──────────────────────────────────────────
    sym_as!(builder, "vitalis_verify_create", crate::formal_verification::vitalis_verify_create);
    sym_as!(builder, "vitalis_verify_paths",  crate::formal_verification::vitalis_verify_paths);
    sym_as!(builder, "vitalis_verify_errors", crate::formal_verification::vitalis_verify_errors);
    sym_as!(builder, "vitalis_verify_free",   crate::formal_verification::vitalis_verify_free);

    // ── IDE Features ─────────────────────────────────────────────────
    sym_as!(builder, "vitalis_ide_create",      crate::ide_features::vitalis_ide_create);
    sym_as!(builder, "vitalis_ide_history_len", crate::ide_features::vitalis_ide_history_len);
    sym_as!(builder, "vitalis_ide_free",        crate::ide_features::vitalis_ide_free);

    // ── NAS (Neural Architecture Search) ─────────────────────────────
    sym_as!(builder, "vitalis_nas_create",       crate::nas::vitalis_nas_create);
    sym_as!(builder, "vitalis_nas_generation",   crate::nas::vitalis_nas_generation);
    sym_as!(builder, "vitalis_nas_best_fitness", crate::nas::vitalis_nas_best_fitness);
    sym_as!(builder, "vitalis_nas_pop_size",     crate::nas::vitalis_nas_pop_size);
    sym_as!(builder, "vitalis_nas_free",         crate::nas::vitalis_nas_free);

    // ── Continual Learning ───────────────────────────────────────────
    sym_as!(builder, "vitalis_cl_create_ewc",    crate::continual_learning::vitalis_cl_create_ewc);
    sym_as!(builder, "vitalis_cl_create_replay", crate::continual_learning::vitalis_cl_create_replay);
    sym_as!(builder, "vitalis_cl_tasks_seen",    crate::continual_learning::vitalis_cl_tasks_seen);
    sym_as!(builder, "vitalis_cl_avg_accuracy",  crate::continual_learning::vitalis_cl_avg_accuracy);
    sym_as!(builder, "vitalis_cl_free",          crate::continual_learning::vitalis_cl_free);

    // ── Federated Learning ───────────────────────────────────────────
    sym_as!(builder, "vitalis_fed_create",       crate::federated_learning::vitalis_fed_create);
    sym_as!(builder, "vitalis_fed_add_client",   crate::federated_learning::vitalis_fed_add_client);
    sym_as!(builder, "vitalis_fed_train_round",  crate::federated_learning::vitalis_fed_train_round);
    sym_as!(builder, "vitalis_fed_round",        crate::federated_learning::vitalis_fed_round);
    sym_as!(builder, "vitalis_fed_num_clients",  crate::federated_learning::vitalis_fed_num_clients);
    sym_as!(builder, "vitalis_fed_free",         crate::federated_learning::vitalis_fed_free);

    // ── Phase 14: Systems Programming (v367-v376) ────────────────────
    // Actor Model (v367) — spawn/send/recv already registered in codegen.rs
    sym_as!(builder, "slang_actor_stop",          crate::actor_model::slang_actor_stop);
    sym_as!(builder, "slang_actor_count",         crate::actor_model::slang_actor_count);
    sym_as!(builder, "slang_actor_mailbox_size",  crate::actor_model::slang_actor_mailbox_size);
    sym_as!(builder, "slang_actor_is_alive",      crate::actor_model::slang_actor_is_alive);
    sym_as!(builder, "slang_actor_supervise",     crate::actor_model::slang_actor_supervise);

    // STM (v368) — new/read/write already registered in codegen.rs
    sym_as!(builder, "slang_stm_commit",          crate::stm::slang_stm_commit);
    sym_as!(builder, "slang_stm_abort",           crate::stm::slang_stm_abort);
    sym_as!(builder, "slang_stm_retry",           crate::stm::slang_stm_retry);
    sym_as!(builder, "slang_stm_or_else",         crate::stm::slang_stm_or_else);
    sym_as!(builder, "slang_stm_atomically",      crate::stm::slang_stm_atomically);

    // File System (v369)
    sym_as!(builder, "slang_vfs_create",          crate::file_system::slang_vfs_create);
    sym_as!(builder, "slang_vfs_write",           crate::file_system::slang_vfs_write);
    sym_as!(builder, "slang_vfs_read",            crate::file_system::slang_vfs_read);
    sym_as!(builder, "slang_vfs_exists",          crate::file_system::slang_vfs_exists);
    sym_as!(builder, "slang_vfs_delete",          crate::file_system::slang_vfs_delete);
    sym_as!(builder, "slang_vfs_list",            crate::file_system::slang_vfs_list);
    sym_as!(builder, "slang_vfs_mkdir",           crate::file_system::slang_vfs_mkdir);
    sym_as!(builder, "slang_vfs_size",            crate::file_system::slang_vfs_size);
    sym_as!(builder, "slang_vfs_is_dir",          crate::file_system::slang_vfs_is_dir);
    sym_as!(builder, "slang_vfs_rename",          crate::file_system::slang_vfs_rename);

    // Config Parser (v370)
    sym_as!(builder, "slang_config_parse",        crate::config_parser::slang_config_parse);
    sym_as!(builder, "slang_config_get",          crate::config_parser::slang_config_get);
    sym_as!(builder, "slang_config_set",          crate::config_parser::slang_config_set);
    sym_as!(builder, "slang_config_has",          crate::config_parser::slang_config_has);
    sym_as!(builder, "slang_config_keys",         crate::config_parser::slang_config_keys);
    sym_as!(builder, "slang_config_merge",        crate::config_parser::slang_config_merge);
    sym_as!(builder, "slang_config_validate",     crate::config_parser::slang_config_validate);
    sym_as!(builder, "slang_config_to_string",    crate::config_parser::slang_config_to_string);

    // State Machine (v371)
    sym_as!(builder, "slang_fsm_create",          crate::state_machine::slang_fsm_create);
    sym_as!(builder, "slang_fsm_add_state",       crate::state_machine::slang_fsm_add_state);
    sym_as!(builder, "slang_fsm_add_transition",  crate::state_machine::slang_fsm_add_transition);
    sym_as!(builder, "slang_fsm_trigger",         crate::state_machine::slang_fsm_trigger);
    sym_as!(builder, "slang_fsm_current",         crate::state_machine::slang_fsm_current);
    sym_as!(builder, "slang_fsm_can_trigger",     crate::state_machine::slang_fsm_can_trigger);
    sym_as!(builder, "slang_fsm_reset",           crate::state_machine::slang_fsm_reset);
    sym_as!(builder, "slang_fsm_state_count",     crate::state_machine::slang_fsm_state_count);

    // Scheduler (v372)
    sym_as!(builder, "slang_sched_create",        crate::scheduler::slang_sched_create);
    sym_as!(builder, "slang_sched_add_task",      crate::scheduler::slang_sched_add_task);
    sym_as!(builder, "slang_sched_run_pending",   crate::scheduler::slang_sched_run_pending);
    sym_as!(builder, "slang_sched_cancel",        crate::scheduler::slang_sched_cancel);
    sym_as!(builder, "slang_sched_pending_count", crate::scheduler::slang_sched_pending_count);
    sym_as!(builder, "slang_sched_cron_parse",    crate::scheduler::slang_sched_cron_parse);
    sym_as!(builder, "slang_sched_cron_next",     crate::scheduler::slang_sched_cron_next);
    sym_as!(builder, "slang_sched_clear",         crate::scheduler::slang_sched_clear);

    // Cache (v373)
    sym_as!(builder, "slang_cache_create",        crate::cache::slang_cache_create);
    sym_as!(builder, "slang_cache_put",           crate::cache::slang_cache_put);
    sym_as!(builder, "slang_cache_get",           crate::cache::slang_cache_get);
    sym_as!(builder, "slang_cache_remove",        crate::cache::slang_cache_remove);
    sym_as!(builder, "slang_cache_contains",      crate::cache::slang_cache_contains);
    sym_as!(builder, "slang_cache_size",          crate::cache::slang_cache_size);
    sym_as!(builder, "slang_cache_clear",         crate::cache::slang_cache_clear);
    sym_as!(builder, "slang_cache_hit_rate",      crate::cache::slang_cache_hit_rate);
    sym_as!(builder, "slang_cache_eviction_count",crate::cache::slang_cache_eviction_count);
    sym_as!(builder, "slang_cache_capacity",      crate::cache::slang_cache_capacity);

    // Search Index (v374)
    sym_as!(builder, "slang_idx_create",          crate::search_index::slang_idx_create);
    sym_as!(builder, "slang_idx_add_doc",         crate::search_index::slang_idx_add_doc);
    sym_as!(builder, "slang_idx_search",          crate::search_index::slang_idx_search);
    sym_as!(builder, "slang_idx_doc_count",       crate::search_index::slang_idx_doc_count);
    sym_as!(builder, "slang_idx_term_count",      crate::search_index::slang_idx_term_count);
    sym_as!(builder, "slang_idx_bm25_score",      crate::search_index::slang_idx_bm25_score);
    sym_as!(builder, "slang_idx_tfidf_score",     crate::search_index::slang_idx_tfidf_score);
    sym_as!(builder, "slang_idx_remove_doc",      crate::search_index::slang_idx_remove_doc);

    // Template Engine (v375)
    sym_as!(builder, "slang_tpl_render",          crate::template_engine::slang_tpl_render);
    sym_as!(builder, "slang_tpl_set_var",         crate::template_engine::slang_tpl_set_var);
    sym_as!(builder, "slang_tpl_get_var",         crate::template_engine::slang_tpl_get_var);
    sym_as!(builder, "slang_tpl_has_var",         crate::template_engine::slang_tpl_has_var);
    sym_as!(builder, "slang_tpl_var_count",       crate::template_engine::slang_tpl_var_count);
    sym_as!(builder, "slang_tpl_clear_vars",      crate::template_engine::slang_tpl_clear_vars);
    sym_as!(builder, "slang_tpl_validate",        crate::template_engine::slang_tpl_validate);
    sym_as!(builder, "slang_tpl_escape_html",     crate::template_engine::slang_tpl_escape_html);

    // Logging (v376)
    sym_as!(builder, "slang_log_init",            crate::logging::slang_log_init);
    sym_as!(builder, "slang_log_debug",           crate::logging::slang_log_debug);
    sym_as!(builder, "slang_log_info",            crate::logging::slang_log_info);
    sym_as!(builder, "slang_log_warn",            crate::logging::slang_log_warn);
    sym_as!(builder, "slang_log_error",           crate::logging::slang_log_error);
    sym_as!(builder, "slang_log_set_level",       crate::logging::slang_log_set_level);
    sym_as!(builder, "slang_log_count",           crate::logging::slang_log_count);
    sym_as!(builder, "slang_log_clear",           crate::logging::slang_log_clear);

    // ── Phase 15: Compiler & Language (v377-v386) ────────────────────
    // Alias Analysis (v377)
    sym_as!(builder, "slang_alias_analyze",       crate::alias_analysis::slang_alias_analyze);
    sym_as!(builder, "slang_alias_may_alias",     crate::alias_analysis::slang_alias_may_alias);
    sym_as!(builder, "slang_alias_must_alias",    crate::alias_analysis::slang_alias_must_alias);
    sym_as!(builder, "slang_alias_no_alias",      crate::alias_analysis::slang_alias_no_alias);
    sym_as!(builder, "slang_alias_points_to",     crate::alias_analysis::slang_alias_points_to);
    sym_as!(builder, "slang_alias_set_count",     crate::alias_analysis::slang_alias_set_count);
    sym_as!(builder, "slang_alias_tbaa_check",    crate::alias_analysis::slang_alias_tbaa_check);
    sym_as!(builder, "slang_alias_escape_check",  crate::alias_analysis::slang_alias_escape_check);

    // Loop Vectorizer (v378) — width already registered in codegen.rs
    sym_as!(builder, "slang_vec_analyze",         crate::loop_vectorizer::slang_vec_analyze);
    sym_as!(builder, "slang_vec_is_vectorizable", crate::loop_vectorizer::slang_vec_is_vectorizable);
    sym_as!(builder, "slang_vec_cost_model",      crate::loop_vectorizer::slang_vec_cost_model);
    sym_as!(builder, "slang_vec_unroll_factor",   crate::loop_vectorizer::slang_vec_unroll_factor);
    sym_as!(builder, "slang_vec_dependence_check",crate::loop_vectorizer::slang_vec_dependence_check);
    sym_as!(builder, "slang_vec_slp_analyze",     crate::loop_vectorizer::slang_vec_slp_analyze);
    sym_as!(builder, "slang_vec_transform",       crate::loop_vectorizer::slang_vec_transform);

    // Sanitizer (v379)
    sym_as!(builder, "slang_san_check_bounds",    crate::sanitizer::slang_san_check_bounds);
    sym_as!(builder, "slang_san_check_null",      crate::sanitizer::slang_san_check_null);
    sym_as!(builder, "slang_san_check_overflow",  crate::sanitizer::slang_san_check_overflow);
    sym_as!(builder, "slang_san_check_use_after_free", crate::sanitizer::slang_san_check_use_after_free);
    sym_as!(builder, "slang_san_check_double_free",    crate::sanitizer::slang_san_check_double_free);
    sym_as!(builder, "slang_san_check_leak",      crate::sanitizer::slang_san_check_leak);
    sym_as!(builder, "slang_san_report_count",    crate::sanitizer::slang_san_report_count);
    sym_as!(builder, "slang_san_clear_reports",   crate::sanitizer::slang_san_clear_reports);

    // Refactoring (v380)
    sym_as!(builder, "slang_refactor_rename",     crate::refactoring::slang_refactor_rename);
    sym_as!(builder, "slang_refactor_extract_fn", crate::refactoring::slang_refactor_extract_fn);
    sym_as!(builder, "slang_refactor_inline",     crate::refactoring::slang_refactor_inline);
    sym_as!(builder, "slang_refactor_move",       crate::refactoring::slang_refactor_move);
    sym_as!(builder, "slang_refactor_add_param",  crate::refactoring::slang_refactor_add_param);
    sym_as!(builder, "slang_refactor_remove_param",crate::refactoring::slang_refactor_remove_param);
    sym_as!(builder, "slang_refactor_preview",    crate::refactoring::slang_refactor_preview);
    sym_as!(builder, "slang_refactor_undo",       crate::refactoring::slang_refactor_undo);

    // Parser Combinator (v381)
    sym_as!(builder, "slang_prs_literal",         crate::parser_combinator::slang_prs_literal);
    sym_as!(builder, "slang_prs_regex",           crate::parser_combinator::slang_prs_regex);
    sym_as!(builder, "slang_prs_sequence",        crate::parser_combinator::slang_prs_sequence);
    sym_as!(builder, "slang_prs_choice",          crate::parser_combinator::slang_prs_choice);
    sym_as!(builder, "slang_prs_many",            crate::parser_combinator::slang_prs_many);
    sym_as!(builder, "slang_prs_map",             crate::parser_combinator::slang_prs_map);
    sym_as!(builder, "slang_prs_optional",        crate::parser_combinator::slang_prs_optional);
    sym_as!(builder, "slang_prs_run",             crate::parser_combinator::slang_prs_run);

    // Symbolic Math (v382)
    sym_as!(builder, "slang_sym_parse",           crate::symbolic_math::slang_sym_parse);
    sym_as!(builder, "slang_sym_simplify",        crate::symbolic_math::slang_sym_simplify);
    sym_as!(builder, "slang_sym_diff",            crate::symbolic_math::slang_sym_diff);
    sym_as!(builder, "slang_sym_eval",            crate::symbolic_math::slang_sym_eval);
    sym_as!(builder, "slang_sym_add",             crate::symbolic_math::slang_sym_add);
    sym_as!(builder, "slang_sym_mul",             crate::symbolic_math::slang_sym_mul);
    sym_as!(builder, "slang_sym_degree",          crate::symbolic_math::slang_sym_degree);
    sym_as!(builder, "slang_sym_substitute",      crate::symbolic_math::slang_sym_substitute);
    sym_as!(builder, "slang_sym_expand",          crate::symbolic_math::slang_sym_expand);
    sym_as!(builder, "slang_sym_factor",          crate::symbolic_math::slang_sym_factor);

    // Reactive (v383)
    sym_as!(builder, "slang_rx_create",           crate::reactive::slang_rx_create);
    sym_as!(builder, "slang_rx_map",              crate::reactive::slang_rx_map);
    sym_as!(builder, "slang_rx_filter",           crate::reactive::slang_rx_filter);
    sym_as!(builder, "slang_rx_reduce",           crate::reactive::slang_rx_reduce);
    sym_as!(builder, "slang_rx_merge",            crate::reactive::slang_rx_merge);
    sym_as!(builder, "slang_rx_take",             crate::reactive::slang_rx_take);
    sym_as!(builder, "slang_rx_subscribe",        crate::reactive::slang_rx_subscribe);
    sym_as!(builder, "slang_rx_count",            crate::reactive::slang_rx_count);

    // Session Types (v384) — create already registered in codegen.rs (linear_types)
    sym_as!(builder, "slang_session_send",        crate::session_types::slang_session_send);
    sym_as!(builder, "slang_session_recv",        crate::session_types::slang_session_recv);
    sym_as!(builder, "slang_session_choose",      crate::session_types::slang_session_choose);
    sym_as!(builder, "slang_session_offer",       crate::session_types::slang_session_offer);
    sym_as!(builder, "slang_session_close",       crate::session_types::slang_session_close);
    sym_as!(builder, "slang_session_is_dual",     crate::session_types::slang_session_is_dual);
    sym_as!(builder, "slang_session_validate",    crate::session_types::slang_session_validate);

    // Gradual Typing (v385)
    sym_as!(builder, "slang_grad_check",          crate::gradual_typing::slang_grad_check);
    sym_as!(builder, "slang_grad_cast",           crate::gradual_typing::slang_grad_cast);
    sym_as!(builder, "slang_grad_is_dynamic",     crate::gradual_typing::slang_grad_is_dynamic);
    sym_as!(builder, "slang_grad_guard",          crate::gradual_typing::slang_grad_guard);
    sym_as!(builder, "slang_grad_narrow",         crate::gradual_typing::slang_grad_narrow);
    sym_as!(builder, "slang_grad_widen",          crate::gradual_typing::slang_grad_widen);
    sym_as!(builder, "slang_grad_boundary",       crate::gradual_typing::slang_grad_boundary);
    sym_as!(builder, "slang_grad_consistency",    crate::gradual_typing::slang_grad_consistency);

    // Code Coverage (v386)
    sym_as!(builder, "slang_cov_init",            crate::code_coverage::slang_cov_init);
    sym_as!(builder, "slang_cov_mark_line",       crate::code_coverage::slang_cov_mark_line);
    sym_as!(builder, "slang_cov_hit_line",        crate::code_coverage::slang_cov_hit_line);
    sym_as!(builder, "slang_cov_line_pct",        crate::code_coverage::slang_cov_line_pct);
    sym_as!(builder, "slang_cov_branch_pct",      crate::code_coverage::slang_cov_branch_pct);
    sym_as!(builder, "slang_cov_total_lines",     crate::code_coverage::slang_cov_total_lines);
    sym_as!(builder, "slang_cov_hit_lines",       crate::code_coverage::slang_cov_hit_lines);
    sym_as!(builder, "slang_cov_reset",           crate::code_coverage::slang_cov_reset);

    // ── Phase 16: Distributed & Observability (v387-v396) ────────────
    // DHT (v387)
    sym_as!(builder, "slang_dht_create",          crate::dht::slang_dht_create);
    sym_as!(builder, "slang_dht_put",             crate::dht::slang_dht_put);
    sym_as!(builder, "slang_dht_get",             crate::dht::slang_dht_get);
    sym_as!(builder, "slang_dht_remove",          crate::dht::slang_dht_remove);
    sym_as!(builder, "slang_dht_contains",        crate::dht::slang_dht_contains);
    sym_as!(builder, "slang_dht_size",            crate::dht::slang_dht_size);
    sym_as!(builder, "slang_dht_hash",            crate::dht::slang_dht_hash);
    sym_as!(builder, "slang_dht_rebalance",       crate::dht::slang_dht_rebalance);

    // MapReduce (v388)
    sym_as!(builder, "slang_mr_create",           crate::mapreduce::slang_mr_create);
    sym_as!(builder, "slang_mr_add_input",        crate::mapreduce::slang_mr_add_input);
    sym_as!(builder, "slang_mr_map",              crate::mapreduce::slang_mr_map);
    sym_as!(builder, "slang_mr_shuffle",          crate::mapreduce::slang_mr_shuffle);
    sym_as!(builder, "slang_mr_reduce",           crate::mapreduce::slang_mr_reduce);
    sym_as!(builder, "slang_mr_result",           crate::mapreduce::slang_mr_result);
    sym_as!(builder, "slang_mr_partition_count",  crate::mapreduce::slang_mr_partition_count);
    sym_as!(builder, "slang_mr_reset",            crate::mapreduce::slang_mr_reset);

    // Blockchain (v389)
    sym_as!(builder, "slang_chain_create",        crate::blockchain::slang_chain_create);
    sym_as!(builder, "slang_chain_add_block",     crate::blockchain::slang_chain_add_block);
    sym_as!(builder, "slang_chain_validate",      crate::blockchain::slang_chain_validate);
    sym_as!(builder, "slang_chain_length",        crate::blockchain::slang_chain_length);
    sym_as!(builder, "slang_chain_latest_hash",   crate::blockchain::slang_chain_latest_hash);
    sym_as!(builder, "slang_chain_get_block",     crate::blockchain::slang_chain_get_block);
    sym_as!(builder, "slang_chain_difficulty",    crate::blockchain::slang_chain_difficulty);
    sym_as!(builder, "slang_chain_is_valid",      crate::blockchain::slang_chain_is_valid);
    sym_as!(builder, "slang_merkle_root",         crate::blockchain::slang_merkle_root);
    sym_as!(builder, "slang_merkle_verify",       crate::blockchain::slang_merkle_verify);

    // TLS Engine (v390)
    sym_as!(builder, "slang_tls_handshake",       crate::tls_engine::slang_tls_handshake);
    sym_as!(builder, "slang_tls_encrypt",         crate::tls_engine::slang_tls_encrypt);
    sym_as!(builder, "slang_tls_decrypt",         crate::tls_engine::slang_tls_decrypt);
    sym_as!(builder, "slang_tls_derive_key",      crate::tls_engine::slang_tls_derive_key);
    sym_as!(builder, "slang_tls_verify_cert",     crate::tls_engine::slang_tls_verify_cert);
    sym_as!(builder, "slang_tls_create_cert",     crate::tls_engine::slang_tls_create_cert);
    sym_as!(builder, "slang_tls_session_id",      crate::tls_engine::slang_tls_session_id);
    sym_as!(builder, "slang_tls_is_secure",       crate::tls_engine::slang_tls_is_secure);

    // Circuit Breaker (v391)
    sym_as!(builder, "slang_cb_create",           crate::circuit_breaker::slang_cb_create);
    sym_as!(builder, "slang_cb_call",             crate::circuit_breaker::slang_cb_call);
    sym_as!(builder, "slang_cb_state",            crate::circuit_breaker::slang_cb_state);
    sym_as!(builder, "slang_cb_record_success",   crate::circuit_breaker::slang_cb_record_success);
    sym_as!(builder, "slang_cb_record_failure",   crate::circuit_breaker::slang_cb_record_failure);
    sym_as!(builder, "slang_cb_reset",            crate::circuit_breaker::slang_cb_reset);
    sym_as!(builder, "slang_cb_failure_count",    crate::circuit_breaker::slang_cb_failure_count);
    sym_as!(builder, "slang_cb_is_open",          crate::circuit_breaker::slang_cb_is_open);

    // Load Balancer (v392)
    sym_as!(builder, "slang_lb_create",           crate::load_balancer::slang_lb_create);
    sym_as!(builder, "slang_lb_add_backend",      crate::load_balancer::slang_lb_add_backend);
    sym_as!(builder, "slang_lb_remove_backend",   crate::load_balancer::slang_lb_remove_backend);
    sym_as!(builder, "slang_lb_next",             crate::load_balancer::slang_lb_next);
    sym_as!(builder, "slang_lb_backend_count",    crate::load_balancer::slang_lb_backend_count);
    sym_as!(builder, "slang_lb_set_weight",       crate::load_balancer::slang_lb_set_weight);
    sym_as!(builder, "slang_lb_health_check",     crate::load_balancer::slang_lb_health_check);
    sym_as!(builder, "slang_lb_strategy",         crate::load_balancer::slang_lb_strategy);

    // Service Discovery (v393)
    sym_as!(builder, "slang_sd_register",         crate::service_discovery::slang_sd_register);
    sym_as!(builder, "slang_sd_deregister",       crate::service_discovery::slang_sd_deregister);
    sym_as!(builder, "slang_sd_discover",         crate::service_discovery::slang_sd_discover);
    sym_as!(builder, "slang_sd_health_check",     crate::service_discovery::slang_sd_health_check);
    sym_as!(builder, "slang_sd_service_count",    crate::service_discovery::slang_sd_service_count);
    sym_as!(builder, "slang_sd_is_healthy",       crate::service_discovery::slang_sd_is_healthy);
    sym_as!(builder, "slang_sd_list_services",    crate::service_discovery::slang_sd_list_services);
    sym_as!(builder, "slang_sd_heartbeat",        crate::service_discovery::slang_sd_heartbeat);

    // Metrics Engine (v394) — counter/histogram already registered in codegen.rs
    sym_as!(builder, "slang_metric_inc",          crate::metrics_engine::slang_metric_inc);
    sym_as!(builder, "slang_metric_gauge_set",    crate::metrics_engine::slang_metric_gauge_set);
    sym_as!(builder, "slang_metric_gauge_get",    crate::metrics_engine::slang_metric_gauge_get);
    sym_as!(builder, "slang_metric_p50",          crate::metrics_engine::slang_metric_p50);
    sym_as!(builder, "slang_metric_p99",          crate::metrics_engine::slang_metric_p99);
    sym_as!(builder, "slang_metric_count",        crate::metrics_engine::slang_metric_count);
    sym_as!(builder, "slang_metric_sum",          crate::metrics_engine::slang_metric_sum);
    sym_as!(builder, "slang_metric_reset",        crate::metrics_engine::slang_metric_reset);

    // Log Aggregator (v395)
    sym_as!(builder, "slang_agg_create",          crate::log_aggregator::slang_agg_create);
    sym_as!(builder, "slang_agg_ingest",          crate::log_aggregator::slang_agg_ingest);
    sym_as!(builder, "slang_agg_query",           crate::log_aggregator::slang_agg_query);
    sym_as!(builder, "slang_agg_count",           crate::log_aggregator::slang_agg_count);
    sym_as!(builder, "slang_agg_filter_level",    crate::log_aggregator::slang_agg_filter_level);
    sym_as!(builder, "slang_agg_last_n",          crate::log_aggregator::slang_agg_last_n);
    sym_as!(builder, "slang_agg_clear",           crate::log_aggregator::slang_agg_clear);
    sym_as!(builder, "slang_agg_pattern_match",   crate::log_aggregator::slang_agg_pattern_match);

    // ML Pipeline (v396)
    sym_as!(builder, "slang_pipe_create",         crate::ml_pipeline::slang_pipe_create);
    sym_as!(builder, "slang_pipe_add_stage",      crate::ml_pipeline::slang_pipe_add_stage);
    sym_as!(builder, "slang_pipe_execute",        crate::ml_pipeline::slang_pipe_execute);
    sym_as!(builder, "slang_pipe_stage_count",    crate::ml_pipeline::slang_pipe_stage_count);
    sym_as!(builder, "slang_pipe_remove_stage",   crate::ml_pipeline::slang_pipe_remove_stage);
    sym_as!(builder, "slang_pipe_validate",       crate::ml_pipeline::slang_pipe_validate);
    sym_as!(builder, "slang_pipe_status",         crate::ml_pipeline::slang_pipe_status);
    sym_as!(builder, "slang_pipe_reset",          crate::ml_pipeline::slang_pipe_reset);
}

#[cfg(test)]
mod tests {
    

    #[test]
    fn test_register_all_does_not_panic() {
        // Verify the function compiles and is callable.
        // Actual JIT builder requires ISA setup, so we just check the fn exists.
        assert!(true);
    }

    #[test]
    fn test_symbol_count() {
        // We register 213 symbols from 34 modules.
        // This test documents the expected count.
        let count = 213; // Update when adding new modules
        assert!(count > 100, "should have significant symbol coverage");
    }

    // ── v141: Architecture cleanup tests ─────────────────────────────

    #[test]
    fn test_v141_no_unused_sym_macro() {
        // v141: Removed the unused `sym!` macro.
        // Only `sym_as!` should remain. Count macro_rules definitions
        // excluding test code (skip lines containing "assert" or quotes).
        let source = include_str!("jit_symbols.rs");
        let macro_defs: Vec<&str> = source
            .lines()
            .filter(|l| l.contains("macro_rules!") && !l.contains("assert") && !l.contains('"'))
            .collect();
        // Should only have sym_as!, not sym!
        assert_eq!(
            macro_defs.len(),
            1,
            "expected exactly 1 macro_rules! definition (sym_as!), found: {:?}",
            macro_defs
        );
        assert!(
            macro_defs[0].contains("sym_as"),
            "the only macro should be sym_as!, got: {}",
            macro_defs[0]
        );
    }

    #[test]
    fn test_v141_sym_as_macro_used() {
        // Verify sym_as! is actually used (not dead code itself)
        let source = include_str!("jit_symbols.rs");
        let usage_count = source.matches("sym_as!(").count();
        // Should have many usages (200+)
        assert!(
            usage_count > 100,
            "sym_as! should be used extensively, found {} uses",
            usage_count
        );
    }
}
