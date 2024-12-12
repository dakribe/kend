package main

import (
	"fmt"
	"log"
	"net/http"
	"os"

	"github.com/dakribe/kend/internal/postgres"
	"github.com/go-chi/chi/v5"
	"github.com/go-chi/chi/v5/middleware"
	"github.com/joho/godotenv"
)

func main() {
	err := godotenv.Load()
	if err != nil {
		log.Fatal("Error loading .env")
	}

	pool := postgres.NewPostgres(os.Getenv("DATABASE_URL"))
	fmt.Println(pool)
	r := chi.NewRouter()

	r.Use(middleware.Logger)

	r.Get("/", func(w http.ResponseWriter, r *http.Request) {
		w.Write([]byte("Hello world"))
	})

	http.ListenAndServe(":8000", r)
}
