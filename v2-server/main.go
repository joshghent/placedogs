package main

import (
	"errors"
	"fmt"
	"image"
	"image/jpeg"
	"io/fs"
	"io/ioutil"
	"log"
	"math/rand"
	"os"
	"path/filepath"
	"strconv"
	"time"

	"github.com/gofiber/fiber/v2"
	"github.com/nfnt/resize"
)

func getRandomImage(width int, height int, randomImageIndex int) (string, error) {
	// Define the directory where images are stored
	imagesDir := "./images"

	// Use os.ReadDir which is more efficient than ioutil.ReadDir (deprecated)
	files, err := os.ReadDir(imagesDir)
	if err != nil {
			return "", err
	}

	// Collect image file names with a slice preallocation
	imageFiles := make([]string, 0, len(files))
	for _, file := range files {
			if filepath.Ext(file.Name()) == ".jpeg" {
					imageFiles = append(imageFiles, file.Name())
			}
	}

	// Check if there are any images
	if len(imageFiles) == 0 {
		return "", errors.New("no images found in the images directory")
	}

	// Select a random image file
	randomImageIndex %= len(imageFiles)
	randomImage := imageFiles[randomImageIndex]

	// Open the image file
	file, err := os.Open(filepath.Join(imagesDir, randomImage))
	if err != nil {
		return "", err
	}
	defer file.Close()

	// Decode the image
	img, _, err := image.Decode(file)
	if err != nil {
		return "", err
	}

	// Resize the image using the Lanczos resampling algorithm
	resizedImg := resize.Resize(uint(width), uint(height), img, resize.Lanczos3)

	// Create the cache directory if it doesn't exist
	cacheDir := fmt.Sprintf("./cache/%d/%d/%d", randomImageIndex, width, height)
	if err := os.MkdirAll(cacheDir, 0755); err != nil {
		return "", err
	}

	// Save the resized image
	cacheFileName := filepath.Join(cacheDir, randomImage)
	out, err := os.Create(cacheFileName)
	if err != nil {
		return "", err
	}
	defer out.Close()

	// Write the new image to the cache file
	err = jpeg.Encode(out, resizedImg, nil)
	if err != nil {
		return "", err
	}

	// Return the path to the cached file
	return cacheFileName, nil
}

func getImageFromCache(width int, height int, randomImageIndex int) (string, error) {
	// Construct the directory path based on the width and height
	dir := fmt.Sprintf("./cache/%d/%d/%d", randomImageIndex, width, height)

	// Attempt to find an image file in the directory
	var imagePath string
	found := false

	err := filepath.WalkDir(dir, func(path string, d fs.DirEntry, err error) error {
		if err != nil {
				return err
		}

		if !d.IsDir() && filepath.Ext(path) == ".jpeg" {
				imagePath = path
				found = true
				return errors.New("image found") // Any non-nil error to stop WalkDir
		}

		return nil
	})

	if err != nil && !found {
			// Ignore the "image found" error
			return "", fmt.Errorf("no image found in cache for dimensions %dx%d", width, height)
	}

	return imagePath, nil
}

func main() {
	app := fiber.New()

	app.Static("/", "../build")

	// Serve the website
	app.Get("/", func(c *fiber.Ctx) error {
		return c.SendFile("../build/index.html")
	})

	// Health Route

	// Height/Width Resize route
	app.Get("/:width/:height", func(c *fiber.Ctx) error {
		// Check that the width and height are valid
		widthStr := c.Params("width")
		heightStr := c.Params("height")

		width, err := strconv.Atoi(widthStr)
		if err != nil || width <= 0 || width > 3048 {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{
				"error": "Invalid 'width' parameter. It must be a number greater than 0 and less than 3048.",
			})
		}

		height, err := strconv.Atoi(heightStr)
		if err != nil || height <= 0 || height > 3048 {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{
				"error": "Invalid 'height' parameter. It must be a number greater than 0 and less than 3048.",
			})
		}

		imagesDir := "./images"

		// Read the directory entries
		files, err := ioutil.ReadDir(imagesDir)
		if err != nil {
			return c.Status(fiber.StatusInternalServerError).SendString(err.Error())
		}

		rand.Seed(time.Now().UnixNano())
		// Filter and collect image file names
		imageFiles := []string{}
		for _, file := range files {
			if filepath.Ext(file.Name()) == ".jpeg" {
				imageFiles = append(imageFiles, file.Name())
			}
		}

		// Check if there are any images
		if len(imageFiles) == 0 {
			return c.Status(fiber.StatusInternalServerError).SendString("No images found in the images directory")
		}
		randomImageIndex := rand.Intn(len(imageFiles))


		// Get image from the cache
		img, err := getImageFromCache(width, height, randomImageIndex)

		if err != nil {
    // Log the error for debugging purposes
			log.Printf("Cache miss or error: %v", err)
		}

		if img != "" {
			return c.Status(fiber.StatusOK).SendFile(img)
		}

		// If the image is not in the cache, get a random image and resize it
		imagePath, err := getRandomImage(width, height, randomImageIndex)
		if err != nil {
        return c.Status(fiber.StatusInternalServerError).SendString(err.Error())
    }

		return c.SendFile(imagePath)
	})

	app.Get("/health", func(c *fiber.Ctx) error {
		return c.Status(fiber.StatusOK).JSON(fiber.Map{
			"message": "OK",
		})
	})

	fmt.Println("Listening on port 8033\n")
	log.Fatal(app.Listen(":8033"))
}
