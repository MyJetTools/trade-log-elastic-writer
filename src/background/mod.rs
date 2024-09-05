use std::collections::HashMap;

use elastic_client::{ElasticClient, ElasticIndexRotationPattern};
use serde::{Deserialize, Serialize};
use serde_json::json;
use service_sdk::{
    async_trait,
    my_service_bus::abstractions::subscriber::{
        MessagesReader, MySbSubscriberHandleError, SubscriberCallback,
    },
};
use tokio::sync::Mutex;
use trade_log::{contracts::TradeLogSbModel, serde_json::Value};

pub struct TradeLogSbListener {
    elastic: ElasticClient,
    env_source: String,
    last_created_index: Mutex<Option<String>>,
}

impl TradeLogSbListener {
    pub fn new(elastic: ElasticClient, env_source: String) -> Self {
        Self {
            elastic,
            env_source,
            last_created_index: Mutex::new(None),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TradeLogElasticModelDataItem {
    pub key: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TradeLogElasticModel {
    pub date_time_unix_micros: i64,
    pub trader_id: String,
    pub account_id: String,
    pub component: String,
    pub process_id: String,
    pub operation_id: String,
    pub message: String,
    pub env_source: String,
}

#[async_trait::async_trait]
impl SubscriberCallback<TradeLogSbModel> for TradeLogSbListener {
    async fn handle_messages(
        &self,
        messages_reader: &mut MessagesReader<TradeLogSbModel>,
    ) -> Result<(), MySbSubscriberHandleError> {
        while let Some(messages) = messages_reader.get_all() {
            let logs_to_upload = messages
                .map(|x| {
                    let operation = x.take_message();
                    let data: HashMap<String, String> = operation
                        .data
                        .iter()
                        .map(|x| {
                            (
                                format!("dyn_{}", x.key),
                                serde_yaml::to_string(
                                    &serde_yaml::from_str::<serde_yaml::Value>(&x.value).unwrap(),
                                )
                                .unwrap()
                                .to_string(),
                            )
                        })
                        .collect();

                    let elastic_model = TradeLogElasticModel {
                        date_time_unix_micros: operation.date_time_unix_micros / 1000,
                        trader_id: operation.trader_id,
                        account_id: operation.account_id,
                        component: operation.component,
                        process_id: operation.process_id,
                        operation_id: operation.operation_id,
                        message: operation.message,
                        env_source: self.env_source.clone().to_uppercase(),
                    };

                    let mut elastic_model = serde_json::to_value(&elastic_model).unwrap();

                    if let Value::Object(ref mut map) = elastic_model {
                        for (key, value) in data {
                            map.insert(key, Value::String(value));
                        }
                    }

                    elastic_model
                })
                .collect::<Vec<_>>();

            let index_name = &format!("trade_log_{}", self.env_source.to_lowercase());
            let pattern = ElasticIndexRotationPattern::Day;

            let mut index = self.last_created_index.lock().await;

            let current_date_index = self
                .elastic
                .get_index_name_with_pattern(index_name, &pattern);

            if index.is_none() {
                *index = Some(current_date_index.clone());
                init_elastic_trade_log_index(&self.elastic, index_name, &pattern).await;
            }

            if index.clone().unwrap() != current_date_index {
                *index = Some(current_date_index);
                init_elastic_trade_log_index(&self.elastic, index_name, &pattern).await;
            }

            let response = self
                .elastic
                .write_entities(index_name, &pattern, logs_to_upload)
                .await
                .unwrap();

            if response.status_code() != 200 {
                panic!("Elastic error: status {}", response.status_code());
            }
        }

        Ok(())
    }
}

async fn init_elastic_trade_log_index(
    elastic: &ElasticClient,
    index_name: &str,
    index_pattern: &ElasticIndexRotationPattern,
) {
    let mapping = json!({
        "mappings": {
            "properties": {
                "date_time_unix_micros": { "type": "date", "format": "epoch_millis" },
                "trader_id": { "type": "keyword" },
                "account_id": { "type": "keyword" },
                "component": { "type": "keyword" },
                "process_id": { "type": "keyword" },
                "operation_id": { "type": "keyword" },
                "env_source": { "type": "keyword" },
                "message": { "type": "keyword" },
            },
            "dynamic_templates": [
                {
                  "dyn_text_fields": {
                    "match": "dyn_*",
                    "mapping": {
                      "type": "text",
                      "term_vector": "with_positions_offsets"
                    }
                  }
                }
              ]
        }
    });

    let response = elastic
        .create_index_mapping(index_name, index_pattern, mapping)
        .await;

    println!("Create index response: {:#?}", response);
}
