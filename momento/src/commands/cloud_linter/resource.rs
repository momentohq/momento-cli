use serde::Serialize;

use crate::commands::cloud_linter::dynamodb::DynamoDbMetadata;
use crate::commands::cloud_linter::elasticache::ElastiCacheMetadata;
use crate::commands::cloud_linter::metrics::Metric;
use crate::commands::cloud_linter::serverless_elasticache::ServerlessElastiCacheMetadata;

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub(crate) enum Resource {
    DynamoDb(DynamoDbResource),
    ElastiCache(ElastiCacheResource),
    ServerlessElastiCache(ServerlessElastiCacheResource),
}

#[derive(Debug, Serialize, PartialEq)]
pub(crate) enum ResourceType {
    #[serde(rename = "AWS::DynamoDB::GSI")]
    DynamoDbGsi,
    #[serde(rename = "AWS::DynamoDB::Table")]
    DynamoDbTable,
    #[serde(rename = "AWS::Elasticache::RedisNode")]
    ElastiCacheRedisNode,
    #[serde(rename = "AWS::Elasticache::MemcachedNode")]
    ElastiCacheMemcachedNode,
    #[serde(rename = "AWS::Elasticache::Serverless")]
    ServerlessElastiCache,
}

#[derive(Serialize, Debug)]
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

#[derive(Serialize, Debug)]
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

#[derive(Serialize, Debug)]
pub(crate) struct ServerlessElastiCacheResource {
    #[serde(rename = "type")]
    pub(crate) resource_type: ResourceType,
    pub(crate) region: String,
    pub(crate) id: String,
    pub(crate) metrics: Vec<Metric>,
    #[serde(rename = "metricPeriodSeconds")]
    pub(crate) metric_period_seconds: i32,
    pub(crate) metadata: ServerlessElastiCacheMetadata,
}

#[derive(Serialize, Debug)]
pub(crate) struct DataFormat {
    pub(crate) resources: Vec<Resource>,
}
