variable "environment" {
  description = "Namespace for all resources. Eg 'production' 'dev-jane'"
}

variable "location" {
  description = "Azure location"
}

variable "ip_whitelist_insights" {
  description = "List of whitelisted IPs for use with Synapse"
  type        = map(string)
}

variable "ip_whitelist_postgresql" {
  description = "List of allowed IPs for use with PostgreSQL"
  type        = map(string)
}

variable "ad_username" {
  description = "Active Directory Username"
}

variable "ad_object_id" {
  description = "Active Directory Object ID"
}
