package applications

import (
	"context"
	"fmt"
	"time"

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
	ID        uuid.UUID
	Company   string
	Title     string
	CreatedAt time.Time
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

func (s *ApplicationStore) GetAll(ctx context.Context) (*[]Application, error) {
	query := `SELECT * FROM APPLICATIONS`
	rows, err := s.pool.Query(ctx, query)
	if err != nil {
		return &[]Application{}, fmt.Errorf("failed to retrieve applications: %w", err)
	}

	defer rows.Close()

	var applications []Application
	for rows.Next() {
		var app Application
		if err := rows.Scan(&app.ID, &app.Company, &app.Title, &app.CreatedAt); err != nil {
			return nil, fmt.Errorf("failed to scan application row: %w", err)
		}
		applications = append(applications, app)
	}

	return &applications, nil
}
