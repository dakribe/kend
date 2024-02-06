package main

import (
	"github.com/pulumi/pulumi-aws/sdk/v6/go/aws/iam"
	"github.com/pulumi/pulumi-aws/sdk/v6/go/aws/lambda"
	"github.com/pulumi/pulumi/sdk/v3/go/pulumi"
)

type CreateLambdaArgs struct {
	ctx  *pulumi.Context
	name string
	code pulumi.Archive
	role *iam.Role
}

func CreateLambda(args CreateLambdaArgs) (*lambda.Function, error) {
	functionArgs := &lambda.FunctionArgs{
		Handler: pulumi.String("handler"),
		Role:    args.role.Arn,
		Runtime: pulumi.String("provided.al2"),
		Code:    args.code,
	}

	function, err := lambda.NewFunction(args.ctx, args.name, functionArgs)
	if err != nil {
		return &lambda.Function{}, err
	}

	return *&function, nil
}
