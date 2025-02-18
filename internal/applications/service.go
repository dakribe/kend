package applications

import (
	"context"
	"fmt"
)

type ApplicationService struct {
	appStore *ApplicationStore
}

func NewApplicationService(appStore *ApplicationStore) *ApplicationService {
	return &ApplicationService{
		appStore: appStore,
	}
}

type CreateParams struct {
	Title   string
	Company string
}

func (s *ApplicationService) CreateApplication(ctx context.Context, params CreateParams) (*Application, error) {
	app, err := s.appStore.CreateApplication(ctx, params.Company, params.Title)
	if err != nil {
		return &Application{}, fmt.Errorf("Error creating application: %v", err)
	}

	return app, nil
}
