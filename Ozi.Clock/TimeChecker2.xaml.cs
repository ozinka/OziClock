using System;
using System.Collections.Generic;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Shapes;

namespace Ozi.Utilities;

public class LineData
{
    public double Y { get; set; }
    public double X2 { get; set; }
    public double Y2 { get; set; }
}

public partial class TimeChecker2 : Window
{
    private readonly FmMainWindow _fFmMain;

    public TimeChecker2(FmMainWindow fmMain)
    {
        InitializeComponent();
        _fFmMain = fmMain;

        Loaded += Window_Loaded;
    }

    private void GridSplitter_LayoutUpdated(object sender, EventArgs e)
    {
        if (IsMouseOver)
        {
            _fFmMain.NewFmTimeChecker.slTimeChecker.Value =
                rwTop.Height.Value * _fFmMain.NewFmTimeChecker.slTimeChecker.Maximum / rwTop.MaxHeight;
        }
    }

    private void Window_Loaded(object sender, RoutedEventArgs e)
    {
        InitializeCanvases();
    }

    private double GetRegionOffset(string timeZoneId)
    {
        var timeZoneInfo = TimeZoneInfo.FindSystemTimeZoneById(timeZoneId!);

        // Get current time in Kyiv and the region's timezone
        var kyivNow = TimeZoneInfo.ConvertTime(DateTime.UtcNow, TimeZoneInfo.Utc,
            TimeZoneInfo.FindSystemTimeZoneById(App.Settings.MainTimeZone));
        var regionNow = TimeZoneInfo.ConvertTime(DateTime.UtcNow, TimeZoneInfo.Utc, timeZoneInfo);

        // Calculate the offset in hours
        var timeDifference = regionNow - kyivNow;

        // Return the total offset in hours (including fractional hours for half-hour differences)
        return timeDifference.TotalHours;

        // Default to Kyiv's time if region is not found
        return 0;
    }

    private void CreateLines(Canvas canvas, string timeZoneId)
    {
        var lineData = new List<(double y, double x2)>();
        double y = 10;
        double[] pattern = { 25, 15, 15, 20, 15, 15 };

        // Add line data
        for (var i = 0; i < 24; i++) // 24 hours
        {
            foreach (var x2 in pattern)
            {
                lineData.Add((y, x2));
                y += 3;
            }
        }

        // Add one additional line with width 25
        lineData.Add((y, 25));

        // Clear the canvas
        canvas.Children.Clear();

        // Draw the lines
        foreach (var (lineY, x2) in lineData)
        {
            // Left side line
            canvas.Children.Add(new Line
            {
                X1 = 0,
                Y1 = lineY,
                X2 = x2,
                Y2 = lineY,
                Stroke = Brushes.Black,
                StrokeThickness = 1,
                SnapsToDevicePixels = true
            });

            // Right side line (mirrored)
            canvas.Children.Add(new Line
            {
                X1 = 100,
                Y1 = lineY,
                X2 = 100 - x2,
                Y2 = lineY,
                Stroke = Brushes.Black,
                StrokeThickness = 1,
                SnapsToDevicePixels = true
            });
        }

        // Calculate time offset for the region dynamically
        var offset = GetRegionOffset(timeZoneId);

        double labelYOffset = -2; // Starting position for the first label

        // Add time labels considering the timezone offset for the region
        for (var i = 0; i < 25; i++)
        {
            // raw “hour” plus offset (can be fractional, e.g. Pune +5.5)
            var raw = i + offset;

            // wrap into [0,24)
            raw %= 24;
            if (raw < 0)
            {
                raw += 24;
            }

            // split into whole hours + minutes
            var hours = (int)Math.Floor(raw);
            var minutes = (int)Math.Round((raw - hours) * 60);

            // format label
            string timeLabel;
            if (hours == 0 && minutes == 0 && i == 24)
            {
                // show “24” instead of “0”
                timeLabel = "24";
            }
            else if (minutes > 0)
            {
                // e.g. “4:30”
                timeLabel = $"{hours}:{minutes:D2}";
            }
            else
            {
                // whole hour
                timeLabel = hours.ToString();
            }

            // Create the label for the adjusted time
            var label = new Label
            {
                Content = timeLabel,
                FontWeight = FontWeights.Bold,
                HorizontalContentAlignment = HorizontalAlignment.Center,
                Width = 99
            };

            // Add the label to the canvas
            canvas.Children.Add(label);
            label.SetValue(Canvas.LeftProperty, 0.0);
            label.SetValue(Canvas.TopProperty, labelYOffset);


            // Update the Y-coordinate for the next label
            labelYOffset += 18;
        }
    }

    private void InitializeCanvases()
    {
        int i = 0;
        foreach (var timeZoneInfo in App.Settings.TimeZones)
        {
            var (grid, canvas) = CreateCityGrid(i * 100 + 1, timeZoneInfo.Value.Color);
            glRullers.Children.Add(grid);
            CreateLines(canvas, timeZoneInfo.Value.TimeZone);
            i++;
        }

        fmTimeChecker2.Width = i * 100 + 1;
        ColumnDefinition colTopLeft = gdRullers.ColumnDefinitions[0];
        colTopLeft.MaxWidth = (i -1) * 100;
        colTopLeft.Width = new GridLength(App.Settings.MainClockIndex * 100, GridUnitType.Pixel);
    }

    public static (Grid grid, Canvas canvas) CreateCityGrid(double marginLeft, string gradientColor)
    {
        var grid = new Grid
        {
            HorizontalAlignment = HorizontalAlignment.Left,
            Margin = new Thickness(marginLeft, 0, 0, 0),
            Width = 99
        };

        var canvas = new Canvas
        {
            HorizontalAlignment = HorizontalAlignment.Left,
            Width = 99,
            UseLayoutRounding = true
        };

        var linearGradient = new LinearGradientBrush
        {
            StartPoint = new Point(0.5, 0),
            EndPoint = new Point(0.5, 1)
        };
        linearGradient.GradientStops.Add(
            new GradientStop((Color)ColorConverter.ConvertFromString(gradientColor), 0.929));
        linearGradient.GradientStops.Add(new GradientStop((Color)ColorConverter.ConvertFromString("#FF383838"), 1.0));

        canvas.Background = linearGradient;

        grid.Children.Add(canvas);

        return (grid, canvas);
    }
}