package main

import (
	"github.com/pulumi/pulumi-aws/sdk/v6/go/aws/apigatewayv2"
	"github.com/pulumi/pulumi-aws/sdk/v6/go/aws/iam"
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

		timeFunction, err := CreateLambda(CreateLambdaArgs{
			ctx:  ctx,
			name: "timeFunction",
			role: role,
			code: pulumi.NewFileArchive("../timeHandler.zip"),
		})
		if err != nil {
			return err
		}

		testFunction, err := CreateLambda(CreateLambdaArgs{
			ctx:  ctx,
			name: "testFunction",
			role: role,
			code: pulumi.NewFileArchive("../testHandler.zip"),
		})
		if err != nil {
			return err
		}

		nameFunction, err := CreateLambda(CreateLambdaArgs{
			ctx:  ctx,
			name: "nameFunction",
			role: role,
			code: pulumi.NewFileArchive("../nameHandler.zip"),
		})
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

		testRoute, err := CreateApiRoute(ctx, gateway, NewRouteArgs{
			function:  testFunction,
			routeKey:  pulumi.String("GET /"),
			routeName: "test",
		})
		if err != nil {
			return err
		}

		timeRoute, err := CreateApiRoute(ctx, gateway, NewRouteArgs{
			function:  timeFunction,
			routeKey:  pulumi.String("GET /time"),
			routeName: "time",
		})
		if err != nil {
			return err
		}

		nameRoute, err := CreateApiRoute(ctx, gateway, NewRouteArgs{
			function:  nameFunction,
			routeKey:  pulumi.String("GET /name"),
			routeName: "name",
		})
		if err != nil {
			return err
		}

		_, err = apigatewayv2.NewStage(ctx, "apiStage", &apigatewayv2.StageArgs{
			ApiId:      gateway.ID(),
			Name:       pulumi.String("dev"),
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
				&apigatewayv2.StageRouteSettingArgs{
					RouteKey:             nameRoute.RouteKey,
					ThrottlingRateLimit:  pulumi.Float64(10000.00),
					ThrottlingBurstLimit: pulumi.Int(5000),
				},
			},
		})
		if err != nil {
			return err
		}

		apiUrl := pulumi.Sprintf("%s/dev/", gateway.ApiEndpoint)
		ctx.Export("API Endpoint: ", apiUrl)

		return nil
	})
}
