package applications

import (
	"context"
	"fmt"

	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
)

type ApplicationStore struct {
	pool *pgxpool.Pool
}

func NewApplicationStore(pool *pgxpool.Pool) *ApplicationStore {
	return &ApplicationStore{
		pool: pool,
	}
}

type Application struct {
	ID      uuid.UUID
	Company string
	Title   string
}

func (s *ApplicationStore) CreateApplication(ctx context.Context, company, title string) (*Application, error) {
	query := `INSERT INTO applications (company, title) VALUES (@company, @title)
  RETURNING id, company, title`
	args := pgx.NamedArgs{
		"company": company,
		"title":   title,
	}

	var app Application
	err := s.pool.QueryRow(ctx, query, args).Scan(&app.ID, &app.Company, &app.Title)
	if err != nil {
		return &Application{}, fmt.Errorf("failed to create application: %w", err)
	}

	return &app, nil
}
