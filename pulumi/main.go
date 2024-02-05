package main

import (
	"fmt"

	"github.com/pulumi/pulumi-aws/sdk/v6/go/aws/apigatewayv2"
	"github.com/pulumi/pulumi-aws/sdk/v6/go/aws/iam"
	"github.com/pulumi/pulumi-aws/sdk/v6/go/aws/lambda"
	"github.com/pulumi/pulumi/sdk/v3/go/pulumi"
)

func main() {
	pulumi.Run(func(ctx *pulumi.Context) error {
		role, err := iam.NewRole(ctx, "task-exec-role", &iam.RoleArgs{
			AssumeRolePolicy: pulumi.String(`{
				"Version": "2012-10-17",
				"Statement": [{
					"Sid": "",
					"Effect": "Allow",
					"Principal": {
						"Service": "lambda.amazonaws.com"
					},
					"Action": "sts:AssumeRole"
				}]
			}`),
		})

		args := &lambda.FunctionArgs{
			Handler: pulumi.String("handler"),
			Role:    role.Arn,
			Runtime: pulumi.String("provided.al2"),
			Code:    pulumi.NewFileArchive("../timeHandler.zip"),
		}

		testArgs := &lambda.FunctionArgs{
			Handler: pulumi.String("handler"),
			Role:    role.Arn,
			Runtime: pulumi.String("provided.al2"),
			Code:    pulumi.NewFileArchive("../testHandler.zip"),
		}

		timeFunction, err := lambda.NewFunction(
			ctx,
			"timeFunction",
			args)
		if err != nil {
			return err
		}

		testFunction, err := lambda.NewFunction(
			ctx,
			"testFunction",
			testArgs)
		if err != nil {
			return err
		}

		corsConfig := &apigatewayv2.ApiCorsConfigurationArgs{
			AllowMethods: pulumi.StringArray{
				pulumi.String("GET"),
			},
			AllowOrigins: pulumi.StringArray{
				pulumi.String("*"),
			},
		}

		gateway, err := apigatewayv2.NewApi(ctx, "Kend-API", &apigatewayv2.ApiArgs{
			ProtocolType:      pulumi.String("HTTP"),
			CorsConfiguration: corsConfig,
		})
		if err != nil {
			return err
		}

		integration, err := apigatewayv2.NewIntegration(ctx, "timeFunctionIntegration", &apigatewayv2.IntegrationArgs{
			ApiId:                gateway.ID(),
			IntegrationType:      pulumi.String("AWS_PROXY"),
			IntegrationMethod:    pulumi.String("POST"),
			IntegrationUri:       timeFunction.Arn,
			PayloadFormatVersion: pulumi.String("2.0"),
			PassthroughBehavior:  pulumi.String("WHEN_NO_MATCH"),
		})
		if err != nil {
			return err
		}

		testIntegration, err := apigatewayv2.NewIntegration(ctx, "testFunctionIntegration", &apigatewayv2.IntegrationArgs{
			ApiId:                gateway.ID(),
			IntegrationType:      pulumi.String("AWS_PROXY"),
			IntegrationMethod:    pulumi.String("POST"),
			IntegrationUri:       testFunction.Arn,
			PayloadFormatVersion: pulumi.String("2.0"),
			PassthroughBehavior:  pulumi.String("WHEN_NO_MATCH"),
		})
		if err != nil {
			return err
		}

		_, err = lambda.NewPermission(ctx, "timeLambdaPermission", &lambda.PermissionArgs{
			Action:    pulumi.String("lambda:InvokeFunction"),
			Function:  timeFunction,
			Principal: pulumi.String("apigateway.amazonaws.com"),
			SourceArn: gateway.ExecutionArn.ApplyT(func(executionArn string) (string, error) {
				return fmt.Sprintf("%v/*", executionArn), nil
			}).(pulumi.StringOutput),
		})
		if err != nil {
			return err
		}

		_, err = lambda.NewPermission(ctx, "testLambdaPermission", &lambda.PermissionArgs{
			Action:    pulumi.String("lambda:InvokeFunction"),
			Function:  testFunction,
			Principal: pulumi.String("apigateway.amazonaws.com"),
			SourceArn: gateway.ExecutionArn.ApplyT(func(executionArn string) (string, error) {
				return fmt.Sprintf("%v/*", executionArn), nil
			}).(pulumi.StringOutput),
		})
		if err != nil {
			return err
		}

		target := pulumi.Sprintf("integrations/%s", integration.ID())

		timeRoute, err := apigatewayv2.NewRoute(ctx, "timeRoute", &apigatewayv2.RouteArgs{
			ApiId:    gateway.ID(),
			RouteKey: pulumi.String("GET /time"),
			Target:   target,
		})
		if err != nil {
			return err
		}

		testTarget := pulumi.Sprintf("integrations/%s", testIntegration.ID())

		testRoute, err := apigatewayv2.NewRoute(ctx, "testRoute", &apigatewayv2.RouteArgs{
			ApiId:    gateway.ID(),
			RouteKey: pulumi.String("GET /test"),
			Target:   testTarget,
		})
		if err != nil {
			return err
		}

		_, err = apigatewayv2.NewStage(ctx, "apiStage", &apigatewayv2.StageArgs{
			ApiId:      gateway.ID(),
			Name:       pulumi.String("prod"),
			AutoDeploy: pulumi.Bool(true),
			RouteSettings: apigatewayv2.StageRouteSettingArray{
				&apigatewayv2.StageRouteSettingArgs{
					RouteKey:             timeRoute.RouteKey,
					ThrottlingRateLimit:  pulumi.Float64(10000.00),
					ThrottlingBurstLimit: pulumi.Int(5000),
				},
				&apigatewayv2.StageRouteSettingArgs{
					RouteKey:             testRoute.RouteKey,
					ThrottlingRateLimit:  pulumi.Float64(10000.00),
					ThrottlingBurstLimit: pulumi.Int(5000),
				},
			},
		})
		if err != nil {
			return err
		}

		ctx.Export("API Endpoint: ", gateway.ApiEndpoint)

		return nil
	})
}
