pub mod actor_agent;
pub mod agents;
pub mod audit;
pub mod built_in_plugins;
pub mod cognitive;
pub mod crypto;
pub mod data_lifecycle;
pub mod deeptalk;
pub mod embedding;
pub mod entity_attribute_extractor;
pub mod entity_linker;
pub mod entity_relation_extractor;
pub mod entity_summarizer;
pub mod error;
pub mod event_aggregator;
pub mod event_extractor;
pub mod event_storage;
pub mod export;
pub mod http_api;
pub mod input;
pub mod llm_provider;
pub mod models;
pub mod pattern_detector;
pub mod plugin;
pub mod prompt_manager;
pub mod resource_manager;
pub mod schema;
pub mod security_tests;
pub mod view_generator;

pub use agents::{
    Agent, AgentPermissions, AgentRepository, AgentUpdate, MemoryPermission, NewAgent,
};
pub use plugin::{
    EntityFilter, EventFilter, EventSubscription, PluginContext, PluginMemoryInterface,
    PluginMetadata, PluginOutput, PluginResponse, PluginSpec, PluginTimeRange, Statistics, UserPlugin,
};
pub use crypto::{EncryptionManager, SecureBuffer, DEFAULT_KEY_FILE};
pub use embedding::{EmbeddingConfig, EmbeddingGenerator, EMBEDDING_DIM};
pub use entity_attribute_extractor::{Attribute, AttributeType, EntityAttributeExtractor};
pub use entity_linker::EntityLinker;
pub use entity_relation_extractor::{
    EntityRelationExtractor, ExtractedRelation, RelationExtractorConfig, RelationType,
};
pub use entity_summarizer::EntitySummarizer;
pub use error::{DirSoulError, Result};
pub use event_aggregator::{AggregationResult, AggregationType, EventAggregator, TimeRange};
pub use event_extractor::{ExtractedEvent, RuleExtractor, SlmExtractor, TimeParser};
pub use event_storage::EventStorage;
pub use input::{InputProcessor, RawInput};
pub use llm_provider::{
    ChatMessage, ChatResponse, LLMProvider, ModelConfig, ModelProviderFactory,
    OllamaProvider, OpenAICompatibleProvider, extract_response_text,
};
pub use models::{
    ContentType, Entity, EntityRelation, EntityType, NewEntity, NewEntityRelation,
    EventMemory, NewEventMemory, NewRawMemory, RawMemory, UpdateRawMemory,
};
pub use prompt_manager::PromptManager;
pub use cognitive::{
    CognitiveView, NewCognitiveView, StableConcept, NewStableConcept, ViewStatus,
};
pub use pattern_detector::{
    DetectionTimeRange, DetectedPattern, PatternDetector, PatternDetectorConfig,
    PatternDetectionResult, PatternDetectionScheduler, PatternMetadata, PatternType, TrendDirection,
};
pub use view_generator::{ViewGenerator, ViewGeneratorBuilder, ViewGeneratorConfig};
pub use deeptalk::{ConversationContext, DeepTalkPlugin, EmotionalTrend};
pub use actor_agent::EventNotification;
pub use built_in_plugins::{DecisionContext, DecisionPlugin, PsychologyContext, PsychologyPlugin};
pub use audit::{AuditLog, AuditLogRepository, AuditLogger, NewAuditLog, ThreadSafeAuditLogger};
pub use export::{AutoBackupManager, DataExporter, DataImporter, EncryptedDataExport, ImportSummary, UserDataExport};
pub use http_api::{
    ApiChatResponse, ChatRequest, EntityStat, HttpServer, StatsRequest,
    StatsResponse, TimelineEvent, TimelineFilters, TimelineRequest, TimelineResponse,
    TimelineSummary, TimeRangeStats,
};
pub use resource_manager::{
    background_memory_monitor, CircuitBreaker, MemoryUsage, ResourceManager,
    ResourceManagerConfig, ResourceAwareScheduler, ScheduledTask, TaskPriority,
};
pub use data_lifecycle::{
    CompressedData, DataLifecycleManager, DataSummary, DataTier, SummaryStatistics,
    TierDistribution, TieringConfig, ArchiveStats,
};
pub use security_tests::{
    run_security_benchmarks, SecurityBenchmarkResults, SecurityTestResult, SecurityTestSuite,
    SecurityTestSuiteResults,
};