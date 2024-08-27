use std::sync::Arc;

use background::TradeLogSbListener;
use elastic_client::ElasticClientAuth;
use settings::SettingsReader;

mod background;
mod settings;

#[tokio::main]
async fn main() {
    let settings_reader = SettingsReader::new(".mjt").await;
    let settings_reader = Arc::new(settings_reader);
    let settings = settings_reader.get_settings().await;

    let elastic_client = elastic_client::ElasticClient::new(ElasticClientAuth::SingleNode {
        url: settings.elastic_url,
        esecure: Some(settings.esecure),
    })
    .unwrap();

    let mut service_context = service_sdk::ServiceContext::new(settings_reader.clone()).await;
    service_context.register_sb_subscribe(Arc::new(TradeLogSbListener::new(
        elastic_client,
        settings.env_source.clone(),
    )), service_sdk::my_service_bus::abstractions::subscriber::TopicQueueType::Permanent).await;

    println!("App started");
    service_context.start_application().await;
}
