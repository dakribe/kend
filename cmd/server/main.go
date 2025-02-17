package main

import (
	"log"
	"net/http"
	"os"

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

	db := postgres.NewPostgres(os.Getenv("DATABASE_URL"))

	e := echo.New()
	e.Use(middleware.Logger())

	e.GET("/", func(c echo.Context) error {
		return c.String(http.StatusOK, "Hello kend")
	})

	e.Start(":8080")
}
