package main

import (
	"fmt"

	"github.com/pulumi/pulumi-aws/sdk/v6/go/aws/apigatewayv2"
	"github.com/pulumi/pulumi-aws/sdk/v6/go/aws/lambda"
	"github.com/pulumi/pulumi/sdk/v3/go/pulumi"
)

type NewRouteArgs struct {
	function  *lambda.Function
	routeKey  pulumi.String
	routeName string
}

func CreateApiRoute(ctx *pulumi.Context, gateway *apigatewayv2.Api, args NewRouteArgs) (*apigatewayv2.Route, error) {
	integration, err := apigatewayv2.NewIntegration(ctx, args.routeName+"Integration", &apigatewayv2.IntegrationArgs{
		ApiId:                gateway.ID(),
		IntegrationType:      pulumi.String("AWS_PROXY"),
		IntegrationMethod:    pulumi.String("POST"),
		IntegrationUri:       args.function.Arn,
		PayloadFormatVersion: pulumi.String("2.0"),
		PassthroughBehavior:  pulumi.String("WHEN_NO_MATCH"),
	})
	if err != nil {
		return &apigatewayv2.Route{}, nil
	}

	_, err = lambda.NewPermission(ctx, args.routeName+"Permission", &lambda.PermissionArgs{
		Action:    pulumi.String("lambda:InvokeFunction"),
		Function:  args.function,
		Principal: pulumi.String("apigateway.amazonaws.com"),
		SourceArn: gateway.ExecutionArn.ApplyT(func(executionArn string) (string, error) {
			return fmt.Sprintf("%v/*", executionArn), nil
		}).(pulumi.StringOutput),
	})
	if err != nil {
		return &apigatewayv2.Route{}, err
	}

	target := pulumi.Sprintf("integrations/%s", integration.ID())

	route, err := apigatewayv2.NewRoute(ctx, args.routeName+"Route", &apigatewayv2.RouteArgs{
		ApiId:    gateway.ID(),
		RouteKey: pulumi.String(args.routeKey),
		Target:   target,
	})

	return route, err
}
