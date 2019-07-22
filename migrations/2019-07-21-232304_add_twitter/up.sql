CREATE TABLE tweets (
  id SERIAL PRIMARY KEY,
  twitter_tweet_id VARCHAR NOT NULL,
  group_name VARCHAR NOT NULL,
  latitude DOUBLE PRECISION,
  longitude DOUBLE PRECISION,
  favorite_count INT NOT NULL,
  retweet_count INT NOT NULL,
  username VARCHAR,
  lang VARCHAR,
  text VARCHAR NOT NULL,
  tweeted_at TIMESTAMPTZ NOT NULL
)
