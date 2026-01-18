#!/bin/bash

# Check for api key
if [ ! -f jres_api_key.txt ]; then
    echo "Error: jres_api_key.txt not found."
    exit 1
fi

# Read the API key from the file
API_KEY=$(cat jres_api_key.txt)

# JSON data to be sent
JSON_DATA='{
  "minimumRestHours": 0,
  "teamMembers": [
    {
      "name": "Niki",
      "isDriver": true
    },
    {
      "name": "Ayrton",
      "isSpotter": true,
      "isDriver": false
    }
  ],
  "availability": {
    "Niki": {
      "1973-06-09T14:00:00.000Z": "Available",
      "1973-06-09T17:00:00.000Z": "Available"
    }
  },
  "stints": [
    {
      "id": 1,
      "startTime": "1973-06-09T14:37:00.000Z",
      "endTime": "1973-06-09T15:27:16.500Z"
    }
  ]
}'

# Make the curl request
curl -X POST https://json.racing/api/solve \
  -H "Content-Type: application/json" \
  -H "X-API-KEY: $API_KEY" \
  -d "$JSON_DATA" \
  -verbose

echo
