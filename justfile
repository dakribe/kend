build-lambda:
  GOOS=linux go build -o bootstrap cmd/handler/main.go  

zip-lambda:
  zip handler.zip bootstrap
