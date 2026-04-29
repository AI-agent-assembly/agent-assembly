//! Integration tests verifying that the Interceptor emits correct
//! PipelineEvents for each supported LLM provider.

use std::time::SystemTime;

use bytes::Bytes;
use tokio::sync::broadcast;

use aa_proto::assembly::audit::v1::audit_event;
use aa_runtime::pipeline::event::{EnrichedEvent, EventSource};
use aa_runtime::pipeline::PipelineEvent;

use aa_proxy::intercept::detect::LlmApiPattern;
use aa_proxy::intercept::event::ProxyEvent;
use aa_proxy::intercept::Interceptor;

fn make_event(pattern: LlmApiPattern, response_body: &str) -> ProxyEvent {
    ProxyEvent {
        agent_id: Some("integration-agent".into()),
        pattern,
        method: "POST".into(),
        path: "/v1/chat/completions".into(),
        request_body: None,
        response_body: Some(Bytes::from(response_body.to_string())),
        timestamp: SystemTime::now(),
    }
}

fn unwrap_llm_detail(event: PipelineEvent) -> (EnrichedEvent, audit_event::Detail) {
    match event {
        PipelineEvent::Audit(enriched) => {
            let detail = enriched.inner.detail.clone().expect("detail must be set");
            (*enriched, detail)
        }
        other => panic!("expected Audit event, got {other:?}"),
    }
}

#[tokio::test]
async fn openai_response_emits_audit_event_with_llm_detail() {
    let (tx, mut rx) = broadcast::channel(16);
    let interceptor = Interceptor::new(tx);

    let event = make_event(
        LlmApiPattern::OpenAi,
        r#"{"model":"gpt-4o","usage":{"prompt_tokens":100,"completion_tokens":50}}"#,
    );
    interceptor.intercept(&event).await.unwrap();

    let (enriched, detail) = unwrap_llm_detail(rx.try_recv().unwrap());
    assert_eq!(enriched.source, EventSource::Proxy);
    assert_eq!(enriched.agent_id, "integration-agent");
    match detail {
        audit_event::Detail::LlmCall(llm) => {
            assert_eq!(llm.model, "gpt-4o");
            assert_eq!(llm.prompt_tokens, 100);
            assert_eq!(llm.completion_tokens, 50);
            assert_eq!(llm.provider, "openai");
        }
        other => panic!("expected LlmCall, got {other:?}"),
    }
}

#[tokio::test]
async fn anthropic_response_emits_audit_event_with_llm_detail() {
    let (tx, mut rx) = broadcast::channel(16);
    let interceptor = Interceptor::new(tx);

    let event = make_event(
        LlmApiPattern::Anthropic,
        r#"{"model":"claude-3-5-sonnet","usage":{"input_tokens":200,"output_tokens":80}}"#,
    );
    interceptor.intercept(&event).await.unwrap();

    let (enriched, detail) = unwrap_llm_detail(rx.try_recv().unwrap());
    assert_eq!(enriched.source, EventSource::Proxy);
    match detail {
        audit_event::Detail::LlmCall(llm) => {
            assert_eq!(llm.model, "claude-3-5-sonnet");
            assert_eq!(llm.prompt_tokens, 200);
            assert_eq!(llm.completion_tokens, 80);
            assert_eq!(llm.provider, "anthropic");
        }
        other => panic!("expected LlmCall, got {other:?}"),
    }
}

#[tokio::test]
async fn cohere_response_emits_audit_event_with_llm_detail() {
    let (tx, mut rx) = broadcast::channel(16);
    let interceptor = Interceptor::new(tx);

    let event = make_event(
        LlmApiPattern::Cohere,
        r#"{"model":"command-r-plus","message":"hi","meta":{"tokens":{"input_tokens":30,"output_tokens":15}}}"#,
    );
    interceptor.intercept(&event).await.unwrap();

    let (enriched, detail) = unwrap_llm_detail(rx.try_recv().unwrap());
    assert_eq!(enriched.source, EventSource::Proxy);
    match detail {
        audit_event::Detail::LlmCall(llm) => {
            assert_eq!(llm.model, "command-r-plus");
            assert_eq!(llm.prompt_tokens, 30);
            assert_eq!(llm.completion_tokens, 15);
            assert_eq!(llm.provider, "cohere");
        }
        other => panic!("expected LlmCall, got {other:?}"),
    }
}

#[tokio::test]
async fn multiple_llm_calls_produce_separate_events() {
    let (tx, mut rx) = broadcast::channel(16);
    let interceptor = Interceptor::new(tx);

    let event1 = make_event(
        LlmApiPattern::OpenAi,
        r#"{"model":"gpt-4","usage":{"prompt_tokens":10,"completion_tokens":5}}"#,
    );
    let event2 = make_event(
        LlmApiPattern::Anthropic,
        r#"{"model":"claude-3-opus","usage":{"input_tokens":20,"output_tokens":10}}"#,
    );

    interceptor.intercept(&event1).await.unwrap();
    interceptor.intercept(&event2).await.unwrap();

    let (_, detail1) = unwrap_llm_detail(rx.try_recv().unwrap());
    let (_, detail2) = unwrap_llm_detail(rx.try_recv().unwrap());

    match detail1 {
        audit_event::Detail::LlmCall(llm) => assert_eq!(llm.provider, "openai"),
        other => panic!("expected LlmCall, got {other:?}"),
    }
    match detail2 {
        audit_event::Detail::LlmCall(llm) => assert_eq!(llm.provider, "anthropic"),
        other => panic!("expected LlmCall, got {other:?}"),
    }
}
