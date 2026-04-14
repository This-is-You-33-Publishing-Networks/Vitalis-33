//! Vitalis v29.0 — A JIT/AOT-compiled language with profiling & PGO,
//! (Mutex, RwLock, channels, Select, WaitGroup, atomics, scoped tasks, deadlock detection),
//! advanced type inference (Hindley-Milner Algorithm W, unification, bidirectional checking,
//! union/intersection types, flow-sensitive narrowing),
//! documentation generation (doc-comment parser, API model, Markdown/HTML output,
//! cross-references, example extraction, dependency graphs),
//! hygienic macro system, compile-time evaluation, lazy iterator protocol,
//! code formatter, static linter,
//! refinement types, algebraic effect handlers, pattern exhaustiveness checking,
//! or-patterns, non-lexical lifetimes (NLL), async/await, generics, WASM target,
//! LSP IDE support, GPU compute, package management, impl blocks, try/catch,
//! closures with capture, stdlib functions, built-in code evolution, multi-domain
//! algorithm libraries, lifetime annotations, region analysis, effect system,
//! capability types, incremental codegen, hot-reload, self-hosted compiler bootstrap,
//! native AOT compilation, cross-compilation (x86_64, AArch64, RISC-V),
//! and native Cranelift JIT performance.
//!
//! Enterprise-grade release with 82 modules spanning profiling & PGO,
//! type inference, documentation generation, macro system, compile-time
//! evaluation, iterator/generator protocol, code formatting, static linting,
//! refinement types, algebraic effect handlers, pattern exhaustiveness analysis,
//! NLL borrow analysis, async runtimes,
//! type-system generics, WebAssembly compilation, GPU compute shaders, language server
//! protocol, package management, quantum computing, bioinformatics, neuromorphic
//! computation, advanced evolutionary algorithms, physical sciences, lifetime/region
//! analysis, effect systems, AOT compilation, and cross-compilation targets.
//! This library provides the compiler pipeline (lex → parse → type-check → IR → JIT)
//! and a C FFI bridge so Python code (via ctypes) and other languages can compile
//! and execute `.sl` code natively.
//!
//! # Architecture
//!
//! ```text
//! Source (.sl) → Lexer → Parser → AST → TypeChecker → IR → Cranelift JIT → native
//!                                                      ↕           ↕
//!                                                  WASM Target  GPU Compute
//!                                                      ↕
//!                                             C FFI bridge (bridge.rs)
//!                                                      ↕
//!                                             Python (vitalis.py)
//! ```
//!
//! # Module Domains (v29.0 — 82 modules)
//! - **Core Compiler**: lexer, ast, parser, types, ir, codegen, stdlib
//! - **Async Runtime**: async_runtime (executor, tasks, channels, futures)
//! - **Generics**: generics (type params, monomorphization, type inference, bounds)
//! - **Package Manager**: package_manager (SemVer, registry, dependency resolution)
//! - **LSP Server**: lsp (diagnostics, completion, hover, go-to-def, symbols)
//! - **WebAssembly**: wasm_target (module builder, sections, LEB128, validation)
//! - **GPU Compute**: gpu_compute (buffers, kernels, pipelines, shader builder)
//! - **Evolution**: evolution, engine, meta_evolution, optimizer
//! - **Advanced Evolution**: evolution_advanced (DE, PSO, CMA-ES, NSGA-II, MAP-Elites)
//! - **Memory**: memory (engram store)
//! - **Performance**: hotpath, simd_ops
//! - **Signal Processing**: signal_processing (FFT, DSP, filtering)
//! - **Cryptography**: crypto (SHA-256, HMAC, Base64, CRC32)
//! - **Graph Theory**: graph (BFS, DFS, Dijkstra, MST, SCC, PageRank)
//! - **String Algorithms**: string_algorithms (KMP, Levenshtein, Jaro-Winkler)
//! - **Numerical Methods**: numerical (linear algebra, calculus, interpolation)
//! - **Compression**: compression (RLE, Huffman, LZ77, BWT, delta)
//! - **Probability & Statistics**: probability (distributions, regression, tests)
//! - **Quantum Computing**: quantum, quantum_math, quantum_algorithms (Shor, VQE, QAOA, QPE)
//! - **Advanced Mathematics**: advanced_math (number theory, tensors, Galois fields)
//! - **Science & Physics**: science, chemistry_advanced (stat-mech, relativity, QM)
//! - **Bioinformatics**: bioinformatics (DNA/RNA, alignment, epidemiology, kinetics)
//! - **Neuromorphic Computing**: neuromorphic (LIF, Izhikevich, STDP, ESN, NEAT)
//! - **Analytics & Reporting**: analytics (time-series, anomaly detection, forecasting)
//! - **Security Guardrails**: security (validation, injection detection, sandboxing)
//! - **Scoring & Fitness**: scoring (code quality, ELO, Pareto, A/B testing)
//! - **Machine Learning**: ml (k-means, KNN, Naive Bayes, PCA, DBSCAN, LDA)
//! - **Computational Geometry**: geometry (convex hull, Voronoi, Welzl, triangulation)
//! - **Sorting & Searching**: sorting (quicksort, mergesort, radixsort, binary search)
//! - **Automata & Patterns**: automata (Aho-Corasick, Bloom filter, tries, regex)
//! - **Combinatorial Optimization**: combinatorial (knapsack, TSP, simplex, genetic)
//! - **Lifetime Annotations**: lifetimes (region variables, borrow scoping, outlives constraints)
//! - **Effect System**: effects (IO, Net, FS, Async, GPU effects, capability tokens)
//! - **Hot-Reload**: hot_reload (file watcher, incremental compile, live function swap)
//! - **Self-Hosted Bootstrap**: bootstrap (Stage 0/1/2 pipeline, cross-validation)
//! - **Native AOT Compilation**: aot (ObjectModule backend, static linking, standalone executables)
//! - **Cross-Compilation**: cross_compile (x86_64, AArch64, RISC-V targets, ABI configs)
//! - **Non-Lexical Lifetimes**: nll (CFG builder, liveness analysis, NLL regions, conflict detection)
//! - **Effect Handlers**: effect_handlers (algebraic effects, continuations, handler stacks)
//! - **Pattern Analysis**: pattern_exhaustiveness (exhaustiveness, redundancy, or-patterns, witnesses)
//! - **Code Formatter**: formatter (AST-based pretty-printer, configurable style)
//! - **Static Linter**: linter (unused vars, naming, complexity, unreachable code)
//! - **Refinement Types**: refinement_types (dependent types, constraint solver, subtyping)
//! - **Macro System**: macro_system (hygienic macros, derive, token trees, pattern matching)
//! - **Compile-Time Eval**: const_eval (const fn, const generics, static_assert, folding)
//! - **Iterator Protocol**: iterators (lazy iterators, generators, adapters, state machines)
//! - **Structured Concurrency**: concurrency (Mutex, RwLock, channels, Select, WaitGroup, atomics, scoped tasks)
//! - **Type Inference**: type_inference (Hindley-Milner, unification, bidirectional, union/intersection, narrowing)
//! - **Documentation Generation**: documentation (doc-comment parsing, API model, Markdown/HTML output, cross-refs)
//! - **Profiler & PGO**: profiler (execution profiling, call graphs, flame graphs, PGO hints, hot-path detection)
//! - **Memory Pools**: memory_pool (arena, pool, slab, buddy allocators, RC heap with cycle detection)
//! - **FFI Bindgen**: ffi_bindgen (C ABI layout, header generation, TypeScript .d.ts, calling conventions, type marshaling)
//! - **Type Classes & HKTs**: type_classes (kind system, type classes, GADTs, type families, type-level naturals)
//! - **Build System**: build_system (build graph DAG, content-addressed cache, work-stealing scheduler, critical path)
//! - **Benchmarks**: benchmark (micro-benchmarking, statistical analysis, outlier detection, regression testing)

// Library modules expose public API surface (FFI bridge, stdlib builtins, JIT symbols)
// that may not be consumed within the crate itself. Removing this would produce
// hundreds of false-positive warnings for intentionally-exported functions.
#![allow(dead_code)]

// ── Core Compiler Pipeline ───────────────────────────────────────────
pub mod lexer;
pub mod ast;
pub mod parser;
pub mod types;
pub mod ir;
pub mod codegen;
pub mod stdlib;

// ── Async Runtime (v21.0) ────────────────────────────────────────────
pub mod async_runtime;

// ── Generics & Type Parameters (v21.0) ───────────────────────────────
pub mod generics;

// ── Package Manager & Registry (v21.0) ───────────────────────────────
pub mod package_manager;
pub mod supply_chain_security;
pub mod runtime_hardening;
pub mod database_primitives;

// ── LSP Server & IDE Support (v21.0) ─────────────────────────────────
pub mod lsp;

// ── WebAssembly Target (v21.0) ───────────────────────────────────────
pub mod wasm_target;

// ── GPU Compute Backend (v21.0) ──────────────────────────────────────
pub mod gpu_compute;
pub mod gpu_kernel_lowering;
pub mod gpu_backend_codegen;

// ── Evolution & Self-Modification ────────────────────────────────────
pub mod evolution;
pub mod engine;
pub mod meta_evolution;
pub mod optimizer;

// ── Memory ───────────────────────────────────────────────────────────
pub mod memory;

// ── Performance ──────────────────────────────────────────────────────
pub mod hotpath;
pub mod simd_ops;

// ── Multi-Domain Algorithm Libraries (v7.0) ──────────────────────────
pub mod signal_processing;
pub mod crypto;
pub mod graph;
pub mod string_algorithms;
pub mod numerical;
pub mod compression;
pub mod probability;

// ── Quantum & Advanced Mathematics (v9.0) ────────────────────────────
pub mod quantum;
pub mod quantum_math;
pub mod advanced_math;

// ── Science & Physics (v9.0) ─────────────────────────────────────────
pub mod science;

// ── Analytics & Reporting (v9.0) ─────────────────────────────────────
pub mod analytics;

// ── Security Guardrails (v9.0) ───────────────────────────────────────
pub mod security;

// ── Scoring & Fitness Evaluation (v9.0) ──────────────────────────────
pub mod scoring;

// ── Machine Learning, Geometry, Sorting, Automata, Optimization (v10.0)
pub mod ml;
pub mod geometry;
pub mod sorting;
pub mod automata;
pub mod combinatorial;

// ── Quantum Algorithms (v13.0) ────────────────────────────────────────
pub mod quantum_algorithms;

// ── Bioinformatics (v13.0) ────────────────────────────────────────────
pub mod bioinformatics;

// ── Advanced Chemistry & Physics (v13.0) ──────────────────────────────
pub mod chemistry_advanced;

// ── Neuromorphic Computing (v13.0) ────────────────────────────────────
pub mod neuromorphic;

// ── Advanced Evolutionary Computation (v13.0) ─────────────────────────
pub mod evolution_advanced;

// ── FFI Bridge ───────────────────────────────────────────────────────
pub mod bridge;

// ── v22: Borrow Checker & Ownership Analysis ─────────────────────────
pub mod ownership;

// ── v22: Incremental Compilation & Caching ───────────────────────────
pub mod incremental;

// ── v22: Full Trait Dispatch with VTables ────────────────────────────
pub mod trait_dispatch;

// ── v22: Debug Adapter Protocol (DAP) ────────────────────────────────
pub mod dap;

// ── v22: Interactive REPL ────────────────────────────────────────────
pub mod repl;

// ── v22: Lifetime Annotations & Region Analysis ─────────────────────
pub mod lifetimes;

// ── v22: Effect System & Capability Types ───────────────────────────
pub mod effects;

// ── v22: Hot-Reload Engine ──────────────────────────────────────────
pub mod hot_reload;

// ── v22: Self-Hosted Compiler Bootstrap ─────────────────────────────
pub mod bootstrap;

// ── v22: Native AOT Compilation ─────────────────────────────────────
pub mod aot;

// ── v22: Cross-Compilation Targets (ARM, RISC-V) ────────────────────
pub mod cross_compile;

// ── v23: Non-Lexical Lifetimes (NLL) ────────────────────────────────
pub mod nll;

// ── v24: Effect Handlers & Pattern Exhaustiveness ───────────────────
pub mod effect_handlers;
pub mod pattern_exhaustiveness;

// ── v25: Code Formatter, Linter & Refinement Types ──────────────────
pub mod formatter;
pub mod linter;
pub mod refinement_types;

// ── v26: Macro System, Compile-Time Eval & Iterator Protocol ────────
pub mod macro_system;
pub mod const_eval;
pub mod iterators;

// ── v27: Structured Concurrency, Type Inference & Documentation ─────
pub mod concurrency;
pub mod type_inference;
pub mod documentation;

// ── v28: Graphics Engine, Shader Languages, GUI, Creative Coding, Visual Nodes & Charts ──
pub mod graphics_engine;
pub mod shader_lang;
pub mod gui_framework;
pub mod gui_renderer;
pub mod gui_widget;
pub mod gui_layout;
pub mod gui_input;
pub mod gui_theme;
pub mod creative_coding;
pub mod visual_nodes;
pub mod chart_rendering;

// ── v29: Profiler, Memory Pools, FFI Bindgen, Type Classes, Build System & Benchmarks ──
pub mod profiler;
pub mod memory_pool;
pub mod ffi_bindgen;
pub mod type_classes;
pub mod build_system;
pub mod build_ci_toolchain;
pub mod benchmark;
pub mod stdlib_system;
pub mod stdlib_serialization_crypto;
pub mod ecosystem_stability;

// ── v30: Regex Engine, Serialization, Property Testing, Data Structures, Networking & ECS ──
pub mod regex_engine;
pub mod serialization;
pub mod property_testing;
pub mod data_structures;
pub mod networking;
pub mod networking_extensions;
pub mod ecs;

// ── v31: Deep Learning Foundation — Tensor Engine & Autograd ─────────
pub mod tensor;
pub mod autograd;

// ── v32: Neural Network Layers & Training Engine ─────────────────────
pub mod neural_net;
pub mod training_engine;

// ── v33: Transformer Architecture & Tokenization ────────────────────
pub mod transformer;
pub mod tokenizer_engine;

// ── v34: Inference Engine, Model Adaptation & Quantization ──────────
pub mod inference;
pub mod model_adaptation;
pub mod quantization;

// ── v35: Code Intelligence, Program Synthesis & Self-Optimization ───
pub mod code_intelligence;
pub mod program_synthesis;
pub mod self_optimizer;

// ── v36: Autonomous Agents & Reward Modeling (RLHF) ─────────────────
pub mod autonomous_agent;
pub mod reward_model;

// ── v37: Differentiable & Probabilistic Programming ─────────────────
pub mod differentiable;
pub mod probabilistic;

// ── v76-v78: AI-Native Tensor Semantics & Differentiable Control Flow ─
pub mod tensor_type_system;
pub mod autodiff_core;
pub mod diff_control_flow;

// ── v38: Reinforcement Learning & Simulation ────────────────────────
pub mod rl_framework;
pub mod simulation;

// ── v39: Data Pipeline & Experiment Tracking ────────────────────────
pub mod data_pipeline;
pub mod experiment;

// ── v40: Model Serving & AI Observability ───────────────────────────
pub mod model_serving;
pub mod ai_observability;

// ── v41: WASM AOT & WASI Runtime ───────────────────────────────────
pub mod wasm_aot;

// ── v42: Package Registry & Distributed Build ──────────────────────
pub mod distributed_build;

// ── v43: Formal Verification & Advanced IDE ────────────────────────
pub mod formal_verification;
pub mod formal_spec_core;
pub mod verified_type_safety;
pub mod optimizer_equivalence;
pub mod verified_codegen_subset;
pub mod quantum_ir_research;
pub mod quantum_backend_proto;
pub mod evolution_safety_rails;
pub mod autonomous_improvement_lab;
pub mod integration_tests;
pub mod perf_baseline;
pub mod fixpoint_candidate;
pub mod overhaul_validation;
pub mod ide_features;

// ── v44: NAS, Continual Learning & Federated Learning ──────────────
pub mod nas;
pub mod continual_learning;
pub mod federated_learning;

// ── v45: GC & Green Threads ────────────────────────────────────────
pub mod gc;
pub mod green_threads;

// ── v46: Database & KV Store ───────────────────────────────────────
pub mod database;
pub mod kv_store;

// ── v47: Consensus & Distributed Primitives ────────────────────────
pub mod consensus;
pub mod distributed_primitives;

// ── v48: Polyhedral Optimization & Parallel Runtime ────────────────
pub mod polyhedral;
pub mod parallel_runtime;

// ── v49: Tiered JIT ────────────────────────────────────────────────
pub mod tiered_jit;

// ── v50: Dependent Types & Proof Assistant ─────────────────────────
pub mod dependent_types;
pub mod proof_assistant;

// ── v51: Time-Travel Debugging & Distributed Tracing ───────────────
pub mod time_travel_debug;
pub mod tracing;

// ── v52: Package Registry v2 & Documentation Site Generator ────────
pub mod registry_v2;
pub mod doc_site;

// ── v53: Notebook Kernel & Web Playground ──────────────────────────
pub mod notebook;
pub mod playground;

// ── v54: Hardware Synthesis & FPGA Target ──────────────────────────
pub mod hardware_synth;
pub mod fpga_target;

// ── v55: Embedded Systems & RTOS Kernel ────────────────────────────
pub mod embedded;
pub mod rtos;

// ── v56: Cloud Deployment & Service Mesh ───────────────────────────
pub mod cloud_deploy;
pub mod service_mesh;

// ── v57: LLM Compiler Assist & Error Recovery ─────────────────────
pub mod llm_compiler;
pub mod error_recovery;

// ── v58: Computer Vision & Audio Processing ────────────────────────
pub mod vision;
pub mod audio;

// ── v59: Certified Compiler & Abstract Interpretation ──────────────
pub mod certified_compiler;
pub mod abstract_interp;

// ── v61: JIT Symbol Bridge ──────────────────────────────────────────
pub mod jit_symbols;

// ── v60: Self-Hosting v2 — Bootstrap & Meta-Compiler ───────────────
pub mod bootstrap_v2;
pub mod meta_compiler;

// ── v201–v300: Neuromorphic Era ─────────────────────────────────────────────
pub mod spike_engine;
pub mod loihi_sim;
pub mod snn_learning;
pub mod pim_compute;
pub mod brain_models;
pub mod gpu_neuromorphic;
pub mod neuro_applications;
pub mod neuro_evolve;
pub mod snn_hybrid;
pub mod hippocampal_memory;
pub mod neuro_distributed;
pub mod neuro_tooling;
pub mod quantum_neuro;
pub mod neuro_safety;
pub mod neuro_perf;

// v400 Phase B: Neuromorphic Language Primitives
pub mod spike_types;
pub mod neuro_control_flow;
pub mod snn_codegen;
pub mod neuro_memory_model;
pub mod stdlib_neuromorphic;

// Phase C: Self-Evolving Compiler Core
pub mod pass_evolution;
pub mod codegen_learning;
pub mod self_healing;
pub mod feature_synthesis;
pub mod evolution_observatory;

// ── ERA I: Cognitive Compiler (v601–v700) ──────────────────────────────────

// Phase 27: Semantic Understanding Engine (v601–v625)
pub mod semantic_graph;
pub mod concept_extraction;
pub mod code_reasoning;
pub mod intent_understanding;
pub mod natural_language_spec;
pub mod code_analogy;
pub mod temporal_reasoning;
pub mod causal_inference;

// Phase 28: Self-Aware Optimization (v626–v650)
pub mod compiler_introspection;
pub mod adaptive_pipeline;
pub mod workload_prediction;
pub mod compilation_learning;
pub mod architecture_advisor;
pub mod perf_prophecy;
pub mod optimization_invention;

// Phase 29: Autonomous Debugging (v651–v700)
pub mod root_cause_analysis;
pub mod fault_localization;
pub mod auto_fix_engine;
pub mod regression_prevention;
pub mod specification_mining;
pub mod debug_narrative;
pub mod zero_bug_certification;
pub mod bench_evolution;
pub mod multi_objective_evolution;

// ── ERA II: Neural Compilation (v701–v730) ─────────────────────────────────
// Phase 30: Neural Compiler Pipeline
pub mod neural_parser;
pub mod neural_type_inference;
pub mod neural_optimizer;
pub mod neural_register_alloc;
pub mod neural_codegen;
pub mod neural_scheduler;
pub mod neural_compiler_stack;

// Phase D: Autonomous Developer Intelligence
pub mod intent_compiler;
pub mod auto_test_gen;
pub mod auto_doc;
pub mod auto_review;
pub mod adaptive_compiler;
pub mod collab_agent;
pub mod self_improving_agent;

// Phase E: Neuromorphic Runtime Architecture (v500-v529)
pub mod spike_scheduler;
pub mod synaptic_allocator;
pub mod neural_concurrency;
pub mod predictive_exec;
pub mod homeostatic_runtime;

// v301-v366: Compiler Core, Type System, ML/AI, Systems, Security, Testing, Tooling, Ecosystem
pub mod escape_analysis;
pub mod interprocedural;
pub mod row_types;
pub mod linear_types;
pub mod mixture_of_experts;
pub mod distillation;
pub mod gnn;
pub mod diffusion;
pub mod embedding_search;
pub mod connection_pool;
pub mod protobuf;
pub mod event_sourcing;
pub mod stream_processing;
pub mod message_queue;
pub mod cqrs;
pub mod graphql;
pub mod jwt;
pub mod oauth2;
pub mod chaos;
pub mod csp;
pub mod test_runner;
pub mod snapshot_testing;
pub mod fuzzer;
pub mod api_compat;
pub mod migration;
pub mod openapi;
pub mod release;
pub mod plugin_system;

// Phase 14: Systems Programming (v367-v376)
pub mod actor_model;
pub mod stm;
pub mod file_system;
pub mod config_parser;
pub mod state_machine;
pub mod scheduler;
pub mod cache;
pub mod search_index;
pub mod template_engine;
pub mod logging;

// Phase 15: Compiler & Language (v377-v386)
pub mod alias_analysis;
pub mod loop_vectorizer;
pub mod sanitizer;
pub mod refactoring;
pub mod parser_combinator;
pub mod symbolic_math;
pub mod reactive;
pub mod session_types;
pub mod gradual_typing;
pub mod code_coverage;

// Phase 16: Distributed & Observability (v387-v396)
pub mod dht;
pub mod mapreduce;
pub mod blockchain;
pub mod tls_engine;
pub mod circuit_breaker;
pub mod load_balancer;
pub mod service_discovery;
pub mod metrics_engine;
pub mod log_aggregator;
pub mod ml_pipeline;

// ── Era II: Neural Compiler Architecture (v731-v800) ─────────────────
pub mod neuromorphic_os;
pub mod spike_memory_hierarchy;
pub mod cortical_program_layout;
pub mod neural_gc;
pub mod whole_brain_runtime;
pub mod language_evolution;
pub mod language_genome;

// ── Era III: Sentient Compilation (v801-v900) ────────────────────────
pub mod collaboration_mind;
pub mod explanation_engine;
pub mod developer_model;
pub mod theorem_prover;
pub mod project_generator;
pub mod deployment_pipeline;
pub mod incident_response;
pub mod api_evolution;
pub mod documentation_synthesis;
pub mod software_organism;

// ── Era IV: Transcendent Computing (v901-v1000) ─────────────────────
pub mod immune_system;
pub mod ecosystem_runtime;
pub mod morphogenesis;
pub mod dream_compilation;
pub mod consciousness_model;
pub mod quantum_backend_v2;
pub mod hybrid_quantum_classical;
pub mod agi_framework;
pub mod meta_cognition;
pub mod collective_intelligence;
pub mod recursive_self_improvement;
pub mod architecture_transcendence;
pub mod formal_creativity;
pub mod universal_compiler;
pub mod zero_shot_compilation;
pub mod emergent_specifications;
pub mod self_reproducing_compiler;
pub mod evolutionary_singularity;
pub mod computational_consciousness;

// ── ERA V: Post-Singularity Intelligence (v1001-v1066) ────────────────

// Phase 40: Distributed Consciousness
pub mod hive_mind;
pub mod telepathic_link;
pub mod consensus_consciousness;
pub mod swarm_cognition;

// Phase 41: Reality Modeling
pub mod reality_model;
pub mod causal_fabric;
pub mod temporal_weaver;
pub mod entropy_oracle;

// Phase 42: Meta-Cognitive Architecture
pub mod metacognitive_stack;
pub mod attention_allocator;
pub mod cognitive_cache;
pub mod introspection_engine;

// Phase 43: Emergent Behavior
pub mod emergence_detector;
pub mod symbiotic_compiler;
pub mod collective_memory;
pub mod cultural_evolution;
pub mod noosphere;

// Phase 44: Post-Singular Optimization
pub mod trans_optimization;
pub mod hypercomputation;
pub mod omega_point_optimizer;
pub mod strange_loop;
pub mod infinite_regress_resolver;

// ── ERA VI: Reality Engineering (v1067-v1133) ─────────────────────────

// Phase 45: Digital Physics
pub mod digital_physics;
pub mod information_geometry;
pub mod computational_topology;
pub mod phase_transition;

// Phase 46: Simulation Theory
pub mod simulation_substrate;
pub mod nested_reality;
pub mod virtual_physics_engine;
pub mod reality_compiler;

// Phase 47: Causal Manipulation
pub mod causal_rewriter;
pub mod retrocausal_optimizer;
pub mod timeline_surgery;
pub mod probability_collapse;
pub mod determinism_engine;

// Phase 48: Spacetime Computation
pub mod spacetime_compiler;
pub mod relativistic_scheduler;
pub mod dimensional_reduction;
pub mod holographic_memory;
pub mod field_theory_types;

// Phase 49: Universe Primitives
pub mod multiverse_executor;
pub mod cosmological_gc;
pub mod vacuum_state;
pub mod symmetry_compiler;

// ── ERA VII: Universal Synthesis (v1134-v1200) ────────────────────────

// Phase 50: Cross-Paradigm Unification
pub mod paradigm_fusion;
pub mod universal_abstraction;
pub mod polymorphic_compilation;
pub mod abstraction_lattice;

// Phase 51: Knowledge Crystallization
pub mod knowledge_crystal;
pub mod wisdom_extractor;
pub mod pattern_genome;
pub mod insight_propagator;
pub mod compilation_archaeology;

// Phase 52: Unified Type Universe
pub mod type_universe;
pub mod homotopy_types;
pub mod type_topology;
pub mod cubical_types;

// Phase 53: Synthesis Engines
pub mod program_weaver;
pub mod specification_compiler;
pub mod intent_synthesizer;
pub mod proof_driven_codegen;
pub mod example_driven_synthesis;

// Phase 54: Grand Unification
pub mod grand_unified_ir;
pub mod rosetta_transform;
pub mod compilation_field_theory;
pub mod convergence_proof;

// ── ERA VIII: Infinite Horizon (v1201-v1266) ──────────────────────────

// Phase 55: Self-Transcendence
pub mod self_transcendence;
pub mod bootstrap_infinity;
pub mod capability_horizon;
pub mod limit_breaker;

// Phase 56: Godelian Self-Reference
pub mod godel_numbering;
pub mod fixed_point_combinator;
pub mod incompleteness_navigator;
pub mod diagonal_argument;
pub mod self_reference_engine;

// Phase 57: Paradox Resolution
pub mod paradox_resolver;
pub mod paraconsistent_logic;
pub mod undecidability_oracle;
pub mod halting_approximator;

// Phase 58: Transfinite Computation
pub mod ordinal_computation;
pub mod supertask_executor;
pub mod omega_arithmetic;
pub mod cantor_hierarchy;
pub mod zorn_optimizer;

// Phase 59: Omega-Point Convergence
pub mod omega_convergence;
pub mod attractor_landscape;
pub mod fixpoint_accelerator;
pub mod asymptotic_perfection;

// ── ERA IX: Vitalis Ascendant (v1267-v1333) ───────────────────────────

// Phase 60: Living Mathematics
pub mod living_mathematics;
pub mod mathematical_organism;
pub mod proof_of_life;
pub mod autopoietic_compiler;

// Phase 61: Autonomous Creative Force
pub mod creative_synthesis;
pub mod aesthetic_compiler;
pub mod invention_engine;
pub mod inspiration_model;
pub mod generative_abstraction;

// Phase 62: Self-Sustaining Ecosystem
pub mod ecosystem_genesis;
pub mod symbiotic_toolchain;
pub mod resource_ecology;
pub mod homeostatic_ecosystem;

// Phase 63: Transcendent Identity
pub mod identity_kernel;
pub mod legacy_continuum;
pub mod version_consciousness;
pub mod philosophical_core;
pub mod telos_engine;

// Phase 64: The Ascension
pub mod vitalis_ascendant;
pub mod eternal_compiler;
pub mod genesis_protocol;
pub mod vitalis_v1333;

// ── The Omega Module (always last) ────────────────────────────────────
pub mod vitalis_omega;
