---
title: Implementing a MongoDB CRUD API Using Go Generics
tags: [go, coding]
date: 2024-05-11
slug: mongodb-crud-api-go-generics
---

I had to write a simple CRUD REST API for a project I'm working on that contains quite a bit of database models in MongoDB. I started writing a controller and a service for each model, but I quickly realized that I was writing the same code over and over again.

Surely there must be a better way to do this.

## Go Generics to the Rescue

Go 1.18 introduced [generics](https://go.dev/doc/tutorial/generics), which allows you to write functions and data structures that can work with any type.

This is perfect for my use case. I can write a generic controller and service that can work with any model, as long as the input and output are the same as the model struct. This of course won't work for every use case, but for simple CRUD resources, it's perfect.

## The Models

Let's start by defining a simple model named `Post` under `models/models.go`:

```go
package models

import "go.mongodb.org/mongo-driver/bson/primitive"

type Post struct {
  ID    primitive.ObjectID `bson:"_id,omitempty" json:"id,omitempty"`
  Title string             `bson:"title" json:"title"`
  Body  string             `bson:"body" json:"body"`
}
```

Note that each field has a `bson` tag for MongoDB and a `json` tag for the API.

## A Naive Approach

Here's what a naive controller and service would look like for the `Post` model:

```go
package controllers


type PostsController struct {
  service *services.PostsService
}

func NewPostsController(service *services.PostsService) *PostsController {
  return &PostsController{service}
}
```

And the service:

```go

type PostsService struct {
  db *mongo.Database
}

func NewPostsService(db *mongo.Database) *PostsService {
  return &PostsService{db}
}

func (s *PostsService) List(ctx context.Context) ([]models.Post, error) {
  // -- snip --
}

func (s *PostsService) Create(ctx context.Context, post *models.Post) (*models.Post, error) {
  // -- snip --
}

func (s *PostsService) Get(ctx context.Context, id string) (*models.Post, error) {
  // -- snip --
}

func (s *PostsService) Update(ctx context.Context, id string, post *models.Post) (*models.Post, error) {
  // -- snip --
}

func (s *PostsService) Delete(ctx context.Context, id string) error {
  // -- snip --
}
```

We can clearly see how repetitive this code is. We can do better.

## A Generic Approach

### Service

We'll start out by defining a generic service that can work with any model:

```go
package services

import (
	"context"

	"go.mongodb.org/mongo-driver/bson"
	"go.mongodb.org/mongo-driver/bson/primitive"
	"go.mongodb.org/mongo-driver/mongo"
)

type CrudService[T any] struct {
	db         *mongo.Database
	collection string
}

func NewCrudService[T any](db *mongo.Database, collection string) *CrudService[T] {
	return &CrudService[T]{
		db,
		collection,
	}
}

func (s *CrudService[T]) List(ctx context.Context) ([]T, error) {
	cursor, err := s.db.Collection(s.collection).Find(ctx, bson.D{})
	if err != nil {
		return nil, err
	}
	var results []T
	if err = cursor.All(ctx, &results); err != nil {
		return nil, err
	}

	return results, nil
}

func (s *CrudService[T]) Create(ctx context.Context, input T) (primitive.ObjectID, error) {
	result, err := s.db.Collection(s.collection).InsertOne(ctx, input)
	if err != nil {
		return primitive.ObjectID{}, err
	}
	resultId, ok := result.InsertedID.(primitive.ObjectID)
	if !ok {
		return primitive.ObjectID{}, err
	}

	return resultId, nil
}

func (s *CrudService[T]) Get(ctx context.Context, id string) (*T, error) {
	objectId, err := primitive.ObjectIDFromHex(id)
	if err != nil {
		return nil, err
	}
	var result T
	err = s.db.Collection(s.collection).FindOne(ctx, bson.M{"_id": objectId}).Decode(&result)
	if err != nil {
		return nil, err
	}

	return &result, nil
}

func (s *CrudService[T]) Update(ctx context.Context, id string, input T) (primitive.ObjectID, error) {
	objectId, err := primitive.ObjectIDFromHex(id)
	if err != nil {
		return primitive.ObjectID{}, err
	}
	filter := bson.M{"_id": objectId}
	update := bson.M{"$set": input}
	_, err = s.db.Collection(s.collection).UpdateOne(ctx, filter, update)
	if err != nil {
		return primitive.ObjectID{}, err
	}

	return objectId, nil
}

func (s *CrudService[T]) Delete(ctx context.Context, id string) error {
	objectId, err := primitive.ObjectIDFromHex(id)
	if err != nil {
		return err
	}
	filter := bson.M{"_id": objectId}
	_, err = s.db.Collection(s.collection).DeleteOne(ctx, filter)
	if err != nil {
		return err
	}

	return nil
}

```

This service accepts the collection name as a parameter so it knows which collection to work with.

```go
type CrudService[T any] struct {
	db         *mongo.Database
	collection string
}
```

This is the generic service struct. It accepts a type `T` which can be any type (in our case, a `Post` model).

```go
func (s *CrudService[T]) List(ctx context.Context) ([]T, error) {
```

The functions are defined with the same type `T` that the service struct accepts, so in this context everywhere you see `T`, think of it as the `Post` model.

### Controller

As for the controller, we will define a generic controller that will contain the service:

```go
package controllers

type CrudController[T any] struct {
	crudService *services.CrudService[T]
}

func NewCrudController[T any](group *echo.Group, db *mongo.Database, collection string) *CrudController[T] {
	c := &CrudController[T]{
		services.NewCrudService[T](db, collection),
	}

	group.GET("", c.List)
	group.POST("", c.Create)
	group.GET("/:id", c.Get)
	group.PUT("/:id", c.Update)
	group.DELETE("/:id", c.Delete)

	return c
}

func (c *CrudController[T]) List(ctx echo.Context) error {
	results, err := c.crudService.List(ctx.Request().Context())
	if err != nil {
		return err
	}

	return ctx.JSON(200, results)
}

func (c *CrudController[T]) Create(ctx echo.Context) error {
	var input T
	if err := ctx.Bind(&input); err != nil {
		return err
	}
	result, err := c.crudService.Create(ctx.Request().Context(), input)
	if err != nil {
		return err
	}

	return ctx.JSON(201, result)
}

func (c *CrudController[T]) Get(ctx echo.Context) error {
	id := ctx.Param("id")
	result, err := c.crudService.Get(ctx.Request().Context(), id)
	if err != nil {
		return err
	}

	return ctx.JSON(200, result)
}

func (c *CrudController[T]) Update(ctx echo.Context) error {
	id := ctx.Param("id")
	var input T
	if err := ctx.Bind(&input); err != nil {
		return err
	}
	result, err := c.crudService.Update(ctx.Request().Context(), id, input)
	if err != nil {
		return err
	}

	return ctx.JSON(200, result)
}

func (c *CrudController[T]) Delete(ctx echo.Context) error {
	id := ctx.Param("id")
	err := c.crudService.Delete(ctx.Request().Context(), id)
	if err != nil {
		return err
	}

	return ctx.NoContent(200)
}
```

This controller registers the routes for the CRUD operations and calls the service methods. Note that I'm using Echo as my web framework of choice, but this will work with any other web framework or router.

## Tying It All Together

In the main file, where we start our HTTP server and connect to the database, we can now use the generic controller to create a controller for the `Post` model:

```go
func main() {
	// -- snip --

	// Controllers
	apiGroup := e.Group("/api")
	controllers.NewCrudController[models.Post](apiGroup.Group("/posts"), db, "posts")
	// as many controllers as you want...

	// -- snip --
}
```

Here you can see how scalable this approach is. You can create as many controllers as you want for different models, and you don't have to write any more service or controller code, unless you need to add custom logic.

Generics are a powerful feature that can help you write more scalable and maintainable code. In this example, we used generics to create a generic CRUD service and controller that can work with any model. This approach is perfect for simple CRUD resources, but it might not work for more complex use cases.

This is a simple example, and I can easily see use cases where the controller and service can be extended to handle more complex logic like pagination, filtering, and more.
