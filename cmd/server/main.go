package main

import (
	"log"
	"net/http"
	"os"

	"github.com/dakribe/kend/internal/applications"
	"github.com/dakribe/kend/internal/postgres"
	"github.com/joho/godotenv"
	"github.com/labstack/echo/v4"
	"github.com/labstack/echo/v4/middleware"
)

func main() {
	err := godotenv.Load()
	if err != nil {
		log.Fatal("Error loading .env file")
	}

	pool := postgres.NewPostgres(os.Getenv("DATABASE_URL"))
	appStore := applications.NewApplicationStore(pool)
	appSvc := applications.NewApplicationService(appStore)

	e := echo.New()
	e.Use(middleware.Logger())

	applications.NewAppliationRoutes(appSvc).Register(e)
	e.GET("/", func(c echo.Context) error {
		return c.String(http.StatusOK, "Hello kend")
	})

	e.Start(":8080")
}
