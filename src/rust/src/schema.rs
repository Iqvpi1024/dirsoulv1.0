// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "vector"))]
    pub struct Vector;
}

diesel::table! {
    agents (agent_id) {
        agent_id -> Uuid,
        user_id -> Text,
        name -> Text,
        agent_type -> Text,
        version -> Text,
        description -> Nullable<Text>,
        author -> Text,
        permissions -> Jsonb,
        is_active -> Bool,
        is_builtin -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        last_used_at -> Nullable<Timestamptz>,
        metadata -> Nullable<Jsonb>,
        tags -> Nullable<Jsonb>,
    }
}

diesel::table! {
    audit_logs (id) {
        id -> Int4,
        user_id -> Text,
        action -> Text,
        target -> Text,
        timestamp -> Timestamptz,
        success -> Bool,
        error_message -> Nullable<Text>,
        result_count -> Nullable<Int4>,
        ip_address -> Nullable<Text>,
        metadata -> Nullable<Jsonb>,
    }
}

diesel::table! {
    cognitive_views (view_id) {
        view_id -> Uuid,
        user_id -> Text,
        hypothesis -> Text,
        view_type -> Text,
        description -> Nullable<Text>,
        derived_from -> Jsonb,
        evidence_count -> Int4,
        confidence -> Float8,
        validation_count -> Int4,
        last_validated_at -> Nullable<Timestamptz>,
        status -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        expires_at -> Timestamptz,
        promoted_to -> Nullable<Uuid>,
        source -> Text,
        tags -> Nullable<Jsonb>,
        metadata -> Nullable<Jsonb>,
        counter_evidence -> Jsonb,
        counter_evidence_count -> Int4,
    }
}

diesel::table! {
    entities (entity_id) {
        entity_id -> Uuid,
        user_id -> Text,
        canonical_name -> Text,
        entity_type -> Text,
        attributes -> Nullable<Jsonb>,
        first_seen -> Timestamptz,
        last_seen -> Timestamptz,
        occurrence_count -> Int4,
        confidence -> Float8,
    }
}

diesel::table! {
    entity_relations (relation_id) {
        relation_id -> Uuid,
        user_id -> Text,
        source_entity_id -> Uuid,
        target_entity_id -> Uuid,
        relation_type -> Text,
        confidence -> Float8,
        first_seen -> Timestamptz,
        last_seen -> Timestamptz,
        strength -> Float8,
    }
}

diesel::table! {
    event_memories (event_id) {
        event_id -> Uuid,
        memory_id -> Uuid,
        user_id -> Text,
        timestamp -> Timestamptz,
        actor -> Nullable<Text>,
        action -> Text,
        target -> Text,
        quantity -> Nullable<Float8>,
        unit -> Nullable<Text>,
        confidence -> Float8,
        extractor_version -> Nullable<Text>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Vector;

    raw_memories (memory_id) {
        memory_id -> Uuid,
        user_id -> Text,
        created_at -> Timestamptz,
        content_type -> Text,
        content -> Nullable<Text>,
        encrypted -> Nullable<Bytea>,
        metadata -> Nullable<Jsonb>,
        embedding -> Nullable<Vector>,
    }
}

diesel::table! {
    stable_concepts (concept_id) {
        concept_id -> Uuid,
        user_id -> Text,
        canonical_name -> Text,
        display_name -> Text,
        concept_type -> Text,
        description -> Nullable<Text>,
        definition -> Jsonb,
        version -> Int4,
        parent_concept_id -> Nullable<Uuid>,
        is_deprecated -> Bool,
        promoted_from -> Nullable<Uuid>,
        promoted_at -> Timestamptz,
        promotion_confidence -> Float8,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        deprecated_at -> Nullable<Timestamptz>,
        access_count -> Int4,
        last_accessed_at -> Nullable<Timestamptz>,
        source -> Text,
        tags -> Nullable<Jsonb>,
        metadata -> Nullable<Jsonb>,
    }
}

diesel::joinable!(cognitive_views -> stable_concepts (promoted_to));
diesel::joinable!(event_memories -> raw_memories (memory_id));

diesel::allow_tables_to_appear_in_same_query!(
    agents,
    audit_logs,
    cognitive_views,
    entities,
    entity_relations,
    event_memories,
    raw_memories,
    stable_concepts,
);
