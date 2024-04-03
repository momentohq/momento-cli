use serde::Serialize;

use crate::commands::cloud_linter::dynamodb::DynamoDbMetadata;
use crate::commands::cloud_linter::elasticache::ElastiCacheMetadata;
use crate::commands::cloud_linter::metrics::Metric;

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum Resource {
    DynamoDb(DynamoDbResource),
    ElastiCache(ElastiCacheResource),
}

#[derive(Debug, Serialize)]
pub(crate) enum ResourceType {
    #[serde(rename = "AWS::DynamoDB::GSI")]
    DynamoDbGsi,
    #[serde(rename = "AWS::DynamoDB::Table")]
    DynamoDbTable,
    #[serde(rename = "AWS::Elasticache::RedisNode")]
    ElastiCacheRedisNode,
    #[serde(rename = "AWS::Elasticache::MemcachedNode")]
    ElastiCacheMemcachedNode,
}

#[derive(Serialize)]
pub(crate) struct DynamoDbResource {
    #[serde(rename = "type")]
    pub(crate) resource_type: ResourceType,
    pub(crate) region: String,
    pub(crate) id: String,
    pub(crate) metrics: Vec<Metric>,
    #[serde(rename = "metricPeriodSeconds")]
    pub(crate) metric_period_seconds: i32,
    pub(crate) metadata: DynamoDbMetadata,
}

#[derive(Serialize)]
pub(crate) struct ElastiCacheResource {
    #[serde(rename = "type")]
    pub(crate) resource_type: ResourceType,
    pub(crate) region: String,
    pub(crate) id: String,
    pub(crate) metrics: Vec<Metric>,
    #[serde(rename = "metricPeriodSeconds")]
    pub(crate) metric_period_seconds: i32,
    pub(crate) metadata: ElastiCacheMetadata,
}

#[derive(Serialize)]
pub(crate) struct DataFormat {
    pub(crate) resources: Vec<Resource>,
}
