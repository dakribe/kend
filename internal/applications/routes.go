package applications

import (
	"fmt"
	"net/http"

	"github.com/labstack/echo/v4"
)

type ApplicationRoutes struct {
	appSvc *ApplicationService
}

func NewAppliationRoutes(appSvc *ApplicationService) *ApplicationRoutes {
	return &ApplicationRoutes{
		appSvc: appSvc,
	}
}

type CreateReq struct {
	Title   string `json:"title"`
	Company string `json:"company"`
}

func (r *ApplicationRoutes) Register(e *echo.Echo) {
	e.GET("/all", r.getAll)
	e.POST("/create", r.create)
}

func (r *ApplicationRoutes) create(c echo.Context) error {
	var createReq CreateReq
	err := c.Bind(&createReq)
	if err != nil {
		return c.String(http.StatusBadRequest, "bad reqeust")
	}

	app, err := r.appSvc.CreateApplication(c.Request().Context(), CreateParams{
		Title:   createReq.Title,
		Company: createReq.Company,
	})
	if err != nil {
		return c.String(http.StatusInternalServerError, "server error")
	}

	return c.JSON(http.StatusOK, app)
}

func (r *ApplicationRoutes) getAll(c echo.Context) error {
	apps, err := r.appSvc.GetApplications(c.Request().Context())
	fmt.Println(err)
	if err != nil {
		return c.String(http.StatusInternalServerError, "server error")
	}

	return c.JSON(http.StatusOK, apps)
}
