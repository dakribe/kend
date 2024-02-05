package main

import (
	"context"
	"time"

	"github.com/aws/aws-lambda-go/events"
	"github.com/aws/aws-lambda-go/lambda"
)

func handler(ctx context.Context, request events.APIGatewayProxyRequest) (events.APIGatewayProxyResponse, error) {
	time := time.Now()

	currentTime := time.Format("2006-01-02 15:04:05")

	return events.APIGatewayProxyResponse{
		Body:       currentTime,
		StatusCode: 200,
	}, nil
}

func main() {
	lambda.Start(handler)
}
