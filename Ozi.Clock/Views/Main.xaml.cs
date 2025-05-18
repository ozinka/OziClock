using System;
using System.Linq;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Interop;
using System.Windows.Media;
using System.Windows.Media.Animation;
using System.Windows.Media.Imaging;
using System.Windows.Threading;
using Ozi.Utilities.ViewModels;

namespace Ozi.Utilities.Views;

// TODO: fix Always on top

public partial class FmMainWindow
{
    //Params related to force to topmost
    private const int HwndTopmost = -1;
    private const uint SwpNosize = 0x0001;
    private const uint SwpNomove = 0x0002;
    private const uint SwpNoactivate = 0x0010;
    private const int HwndNotopmost = -2;

    private const int FoldedHeight = 29;
    private const int UnfoldedHeight = 62;
    private bool _isFolded;
    public readonly Slider Slider;
    private readonly Rulers _rulers;
    private DateTime _localTime;
    private DispatcherTimer? _timeTimer;
    private bool _isMouseOver;
    private bool _isWindowFocused;

    // P/Invoke declaration for SetWindowPos - required for making app always on top
    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetWindowPos(IntPtr hWnd, IntPtr hWndInsertAfter, int x, int y, int cx, int cy,
        uint uFlags);

    // Constructor
    public FmMainWindow()
    {
        InitializeComponent();
        // CommonTimeZones.ShowTimeZones();
        Slider = new Slider(this);
        _rulers = new Rulers(this);

        // Initial opacity from settings
        Opacity = App.Settings.Opacity;
    }

    private void fmMain_LocationChanged(object sender, EventArgs e)
    {
        _rulers.Left = Left;
        _rulers.Top = Top + Height;
        Slider.Left = Left;
        Slider.Top = Top + Height + _rulers.Height;
    }

    private void fmMain_Loaded(object sender, RoutedEventArgs e)
    {
        init_Timer();
        read_Config();
        TtTick(this, EventArgs.Empty);

        App.Clocks.ForEach(clock => GdMain.Children.Add(clock.OsGrid));

        CreateContextMenu();
        Activated += (s, e) =>
        {
            _isWindowFocused = true;
            UpdateOpacity();
        };

        Deactivated += (s, e) =>
        {
            if (!_rulers.IsVisible)
            {
                _isWindowFocused = false;
                UpdateOpacity();
            }
        };
    }

    private MenuItem _itemMoveLeft;
    private MenuItem _itemMoveRight;
    private MenuItem _itemMakeMain;
    private MenuItem _itemRemove;
    private MenuItem _itemFold;
    private MenuItem _itemShowRulers;
    private MenuItem _itemClock;

    private void CreateContextMenu()
    {
        var mainMenu = new ContextMenu();
        mainMenu.Opened += ContextMenu_Opened;

        _itemClock = new MenuItem
        {
            Header = new TextBlock
            {
                Text = "Clock",
                FontWeight = FontWeights.Bold
            }
        };
        var imageClock = new Image
        {
            Source = new BitmapImage(new Uri("pack://application:,,,/Assets/clock.ico", UriKind.Absolute)),
        };
        _itemClock.Icon = imageClock;
        mainMenu.Items.Add(_itemClock);

        mainMenu.Items.Add(new Separator());

        var itemEdit = new MenuItem { Header = "Edit" };
        var imageEdit = new Image
        {
            Source = new BitmapImage(new Uri("pack://application:,,,/Assets/edit2.ico", UriKind.Absolute)),
        };
        itemEdit.Icon = imageEdit;
        itemEdit.Click += ItemEditClick;
        _itemClock.Items.Add(itemEdit);

        _itemMoveLeft = new MenuItem { Header = "Move Left" };
        var imageLeft = new Image
        {
            Source = new BitmapImage(new Uri("pack://application:,,,/Assets/left.ico", UriKind.Absolute)),
        };
        _itemMoveLeft.Icon = imageLeft;
        _itemMoveLeft.Click += ItemMoveLeftOnClick;
        _itemClock.Items.Add(_itemMoveLeft);

        _itemMoveRight = new MenuItem { Header = "Move Right" };
        var imageRight = new Image
        {
            Source = new BitmapImage(new Uri("pack://application:,,,/Assets/right.ico", UriKind.Absolute)),
        };
        _itemMoveRight.Icon = imageRight;
        _itemMoveRight.Click += ItemMoveRightOnClick;
        _itemClock.Items.Add(_itemMoveRight);

        _itemMakeMain = new MenuItem { Header = "Make Main" };
        var imageMain = new Image
        {
            Source = new BitmapImage(new Uri("pack://application:,,,/Assets/main4.ico", UriKind.Absolute)),
        };
        _itemMakeMain.Icon = imageMain;
        _itemMakeMain.Click += ItemMakeMainOnClick;
        _itemClock.Items.Add(_itemMakeMain);

        _itemClock.Items.Add(new Separator());

        _itemRemove = new MenuItem { Header = "Remove" };
        var imageRemove = new Image
        {
            Source = new BitmapImage(new Uri("pack://application:,,,/Assets/remove.ico", UriKind.Absolute)),
        };
        _itemRemove.Icon = imageRemove;
        _itemRemove.Click += MenuItemRemove_Click;
        _itemClock.Items.Add(_itemRemove);

        var itemAdd = new MenuItem { Header = "Add Clock" };
        var imageAdd = new Image
        {
            Source = new BitmapImage(new Uri("pack://application:,,,/Assets/add.ico", UriKind.Absolute)),
        };
        itemAdd.Icon = imageAdd;
        itemAdd.Click += ItemAddOnClick;
        mainMenu.Items.Add(itemAdd);

        _itemFold = new MenuItem { Header = "Fold" };
        var imageFold = new Image
        {
            Source = new BitmapImage(new Uri("pack://application:,,,/Assets/fold.ico", UriKind.Absolute)),
        };
        _itemFold.Icon = imageFold;
        _itemFold.Click += MenuItemFold_Click;
        mainMenu.Items.Add(_itemFold);

        _itemShowRulers = new MenuItem { Header = "Show rulers" };
        var imageRuler = new Image
        {
            Source = new BitmapImage(new Uri("pack://application:,,,/Assets/ruler.ico", UriKind.Absolute)),
        };
        _itemShowRulers.Icon = imageRuler;
        _itemShowRulers.Click += MenuItemShowRulers_Click;
        mainMenu.Items.Add(_itemShowRulers);

        mainMenu.Items.Add(new Separator());

        var itemSettings = new MenuItem { Header = "Settings" };
        var imageSettings = new Image
        {
            Source = new BitmapImage(new Uri("pack://application:,,,/Assets/settings.ico", UriKind.Absolute)),
        };
        itemSettings.Icon = imageSettings;
        itemSettings.Click += MenuItemSettings_Click;
        mainMenu.Items.Add(itemSettings);

        var itemAbout = new MenuItem { Header = "About" };
        var imageAbout = new Image
        {
            Source = new BitmapImage(new Uri("pack://application:,,,/Assets/about.ico", UriKind.Absolute)),
        };
        itemAbout.Icon = imageAbout;
        itemAbout.Click += MenuItemAbout_Click;
        mainMenu.Items.Add(itemAbout);

        mainMenu.Items.Add(new Separator());

        var itemExit = new MenuItem { Header = "Exit" };
        var imageExit = new Image
        {
            Source = new BitmapImage(new Uri("pack://application:,,,/Assets/exit.ico", UriKind.Absolute)),
        };
        itemExit.Icon = imageExit;
        itemExit.Click += MenuItemExit_Click;
        mainMenu.Items.Add(itemExit);

        FmMain.ContextMenu = mainMenu;
    }

    private void ItemAddOnClick(object sender, RoutedEventArgs e)
    {
        App.Clocks.Add(new OsClock("UTC",
            TimeZoneInfo.Utc.Id,
            "#FFFFFFFF",
            false));

        GdMain.Children.Add(App.Clocks[^1].OsGrid);
        if (_rulers.IsLoaded)
        {
            _rulers.GlRulers.Children.Add(App.Clocks[^1].RulerGrid);
            _rulers.UpdateRulers();
            Slider.Size += 1;
        }

        OpenEditWindow(App.Clocks.Count - 1);
    }

    private void OpenEditWindow(int index)
    {
        // Remove topmost
        var windowHandle = new WindowInteropHelper(this).Handle;
        SetWindowPos(windowHandle,
            (IntPtr)HwndNotopmost,
            0, 0, 0, 0,
            SwpNomove | SwpNosize | SwpNoactivate);

        var fmEdit = new Edit(App.Clocks[index])
        {
            Owner = this,
            WindowStartupLocation = WindowStartupLocation.Manual // Set to Manual for custom positioning
        };

        // Get main window and Edit window dimensions
        double mainWindowLeft = this.Left;
        double mainWindowTop = this.Top;
        double mainWindowWidth = this.ActualWidth;
        double mainWindowHeight = this.ActualHeight;
        double editWindowWidth = fmEdit.Width; // 400 as per XAML
        double editWindowHeight = fmEdit.Height; // 200 as per XAML

        // Get screen working area (excludes taskbar)
        var workArea = SystemParameters.WorkArea;

        // Calculate X position: Center of main window
        double editLeft = mainWindowLeft + (mainWindowWidth - editWindowWidth) / 2;

        // Ensure Edit window stays within screen bounds on X-axis
        editLeft = Math.Max(workArea.Left, Math.Min(editLeft, workArea.Right - editWindowWidth));

        // Calculate Y position: Prefer below main window, fallback to above
        double editTop;
        bool enoughSpaceBelow = mainWindowTop + mainWindowHeight + editWindowHeight <= workArea.Bottom;
        if (enoughSpaceBelow)
        {
            // Place below main window
            editTop = mainWindowTop + mainWindowHeight;
        }
        else
        {
            // Place above main window
            editTop = mainWindowTop - editWindowHeight;
            // Ensure it doesn't go above the screen
            editTop = Math.Max(workArea.Top, editTop);
        }

        // Set position
        fmEdit.Left = editLeft;
        fmEdit.Top = editTop;

        fmEdit.Owner = this;
        fmEdit.ShowDialog();

        // Restore topmost
        ForceToTopmost();
    }

    private void ItemEditClick(object sender, RoutedEventArgs e)
    {
        var index = GdMain.Children.IndexOf(_lastRightClickedClock);

        if (_lastRightClickedClock != null)
        {
            OpenEditWindow(index);
        }
    }

    private void ItemMakeMainOnClick(object sender, RoutedEventArgs e)
    {
        var index = GdMain.Children.IndexOf(_lastRightClickedClock);
        if (_lastRightClickedClock != null)
            App.MainClockIndex = index;

        _rulers.UpdateRulers();
        _rulers.UpdateSize();
    }

    private void MenuItemShowRulers_Click(object sender, RoutedEventArgs e)
    {
        if (Slider.Visibility == Visibility.Visible)
        {
            Slider.Hide();
            _rulers.Hide();
            _timeTimer!.Interval = TimeSpan.FromSeconds(1);
        }
        else
        {
            var curTime = TimeZoneInfo.ConvertTimeBySystemTimeZoneId(_localTime, App.MainTimeZoneId);
            // Round to nearest hour
            if (curTime.Minute >= 30)
                curTime = curTime.AddHours(1);
            curTime = new DateTime(curTime.Year, curTime.Month, curTime.Day, curTime.Hour, 0, 0);
            Slider.CurTime = curTime;
            Slider.SlTimeChecker.Value = curTime.Hour * 12; // + (int)(curTime.Minute / 5);
            _rulers.Show();
            Slider.Show();
            _timeTimer!.Interval = TimeSpan.FromMilliseconds(20); // Time reaction to the slider change
        }

        fmMain_MouseLeave(null!, null!);
    }

    private void MenuItemFold_Click(object sender, RoutedEventArgs e)
    {
        if (_isFolded)
        {
            UnFoldMainWindow();
        }
        else
        {
            FoldMainWindow();
        }
    }

    private void ItemMoveRightOnClick(object sender, RoutedEventArgs e)
    {
        var index = GdMain.Children.IndexOf(_lastRightClickedClock);
        if (_lastRightClickedClock != null)
        {
            if (index < App.Clocks.Count - 1)
            {
                var clockToMove = App.Clocks[index];
                var rulerToMove = App.Clocks[index].RulerGrid;

                App.Clocks.RemoveAt(index);
                App.Clocks.Insert(index + 1, clockToMove);

                GdMain.Children.RemoveAt(index);
                GdMain.Children.Insert(index + 1, clockToMove.OsGrid);

                if (_rulers.IsLoaded)
                {
                    _rulers.GlRulers.Children.RemoveAt(index);
                    _rulers.GlRulers.Children.Insert(index + 1, rulerToMove);
                    _rulers.UpdateSize();
                }
            }
        }
    }

    private UIElement? _lastRightClickedClock;

    private void ItemMoveLeftOnClick(object sender, RoutedEventArgs e)
    {
        var index = GdMain.Children.IndexOf(_lastRightClickedClock);
        if (_lastRightClickedClock != null)
        {
            if (index > 0)
            {
                var clockToMove = App.Clocks[index];
                var rulerToMove = App.Clocks[index].RulerGrid;

                App.Clocks.RemoveAt(index);
                App.Clocks.Insert(index - 1, clockToMove);

                GdMain.Children.RemoveAt(index);
                GdMain.Children.Insert(index - 1, clockToMove.OsGrid);

                if (_rulers.IsLoaded)
                {
                    _rulers.GlRulers.Children.RemoveAt(index);
                    _rulers.GlRulers.Children.Insert(index - 1, rulerToMove);
                    _rulers.UpdateSize();
                }
            }
        }
    }

    private void ContextMenu_Opened(object sender, RoutedEventArgs e)
    {
        var pos = Mouse.GetPosition(GdMain); // relative to the grid
        var result = VisualTreeHelper.HitTest(GdMain, pos);


        if (result != null)
        {
            // Traverse up the tree to find the OsClock/Grid you added
            var current = result.VisualHit;
            while (current != null && !(current is Grid && GdMain.Children.Contains((UIElement)current)))
            {
                current = VisualTreeHelper.GetParent(current);
            }

            _lastRightClickedClock = current as UIElement;
            var index = GdMain.Children.IndexOf(_lastRightClickedClock);

            _itemMakeMain.Visibility = (App.Clocks[index].IsMain) ? Visibility.Collapsed : Visibility.Visible;
            _itemMoveLeft.Visibility = (index == 0) ? Visibility.Collapsed : Visibility.Visible;
            _itemMoveRight.Visibility = (index == App.Clocks.Count - 1) ? Visibility.Collapsed : Visibility.Visible;
            _itemRemove.Visibility = (App.Clocks.Count == 1 || App.Clocks[index].IsMain)
                ? Visibility.Collapsed
                : Visibility.Visible;
            _itemFold.Header = _isFolded ? "Unfold" : "Fold";
            _itemShowRulers.Header = _rulers.IsVisible ? "Hide Rulers" : "Show Rulers";
            ((TextBlock)_itemClock.Header).Text = App.Clocks[index].Caption;
        }
    }

    private void MenuItemRemove_Click(object sender, RoutedEventArgs e)
    {
        if (_lastRightClickedClock != null)
        {
            var clockToRemove = App.Clocks
                .FirstOrDefault(c => c.OsGrid == _lastRightClickedClock);

            if (MessageBox.Show("Are you sure you want to delete this clock?", "Confirm Deletion",
                    MessageBoxButton.YesNo, MessageBoxImage.Warning) != MessageBoxResult.Yes)
                return;

            if (clockToRemove != null)
            {
                if (_rulers.IsLoaded)
                {
                    _rulers.GlRulers.Children.Remove(clockToRemove.RulerGrid);
                }

                App.Clocks.Remove(clockToRemove);
                GdMain.Children.Remove(clockToRemove.OsGrid);

                Slider.Size -= 1;
                _rulers.UpdateSize();
            }

            _lastRightClickedClock = null;
        }
    }


    private void read_Config()
    {
        Left = App.Settings.MainWndLeft;
        Top = App.Settings.MainWndTop;
    }

    private void init_Timer()
    {
        _timeTimer = new DispatcherTimer
        {
            Interval = TimeSpan.FromSeconds(1)
        };
        _timeTimer.Tick += TtTick;
        _timeTimer.Start();
    }

    private void TtTick(object sender, EventArgs e)
    {
        var utcNow = DateTime.UtcNow;
        if (Slider.Visibility == Visibility.Visible)
        {
            var localOffset = TimeZoneInfo.Local.GetUtcOffset(utcNow);
            var targetOffset = TimeZoneInfo.FindSystemTimeZoneById(App.MainTimeZoneId).GetUtcOffset(utcNow);

            _localTime = Slider.CurTime.Date + (localOffset - targetOffset);

            _localTime = _localTime.AddMinutes((int)Slider.SlTimeChecker.Value * 5);
            _rulers.RwTop.Height = new GridLength(_rulers.RwTop.MaxHeight *
                Slider.SlTimeChecker.Value / Slider.SlTimeChecker.Maximum);
        }
        else
        {
            _localTime = utcNow;
        }

        foreach (var item in App.Clocks!)
            item.SetTime(_localTime);

        ForceToTopmost();
    }

    private void MenuItemSettings_Click(object sender, RoutedEventArgs e)
    {
        // Remove topmost
        var windowHandle = new WindowInteropHelper(this).Handle;
        SetWindowPos(windowHandle,
            (IntPtr)HwndNotopmost,
            0, 0, 0, 0,
            SwpNomove | SwpNosize | SwpNoactivate);

        var fmFmSettings = new Settings(this)
        {
            WindowStartupLocation = WindowStartupLocation.Manual,
        };

        // Get main window and Edit window dimensions
        double mainWindowLeft = this.Left;
        double mainWindowTop = this.Top;
        double mainWindowWidth = this.ActualWidth;
        double mainWindowHeight = this.ActualHeight;
        double editWindowWidth = fmFmSettings.Width; // 400 as per XAML
        double editWindowHeight = fmFmSettings.Height; // 200 as per XAML

        // Get screen working area (excludes taskbar)
        var workArea = SystemParameters.WorkArea;

        // Calculate X position: Center of main window
        double editLeft = mainWindowLeft + (mainWindowWidth - editWindowWidth) / 2;

        // Ensure Edit window stays within screen bounds on X-axis
        editLeft = Math.Max(workArea.Left, Math.Min(editLeft, workArea.Right - editWindowWidth));

        // Calculate Y position: Prefer below main window, fallback to above
        double editTop;
        bool enoughSpaceBelow = mainWindowTop + mainWindowHeight + editWindowHeight <= workArea.Bottom;
        if (enoughSpaceBelow)
        {
            // Place below main window
            editTop = mainWindowTop + mainWindowHeight;
        }
        else
        {
            // Place above main window
            editTop = mainWindowTop - editWindowHeight;
            // Ensure it doesn't go above the screen
            editTop = Math.Max(workArea.Top, editTop);
        }

        // Set position
        fmFmSettings.Left = editLeft;
        fmFmSettings.Top = editTop;

        fmFmSettings.Owner = this;
        fmFmSettings.ShowDialog();

        // Restore topmost
        ForceToTopmost();
    }

    private void AdjustTimeCheckerPosition()
    {
        _rulers.Left = Left;
        _rulers.Top = Top + Height;
        Slider.Left = Left;
        Slider.Top = Top + Height + _rulers.Height;
    }

    private void MenuItemExit_Click(object sender, RoutedEventArgs e)
    {
        Application.Current.Shutdown();
    }

    private void MenuItemAbout_Click(object sender, RoutedEventArgs e)
    {
        var aboutWindow = new About();
        aboutWindow.Owner = this; // Optional: make it modal to main window
        aboutWindow.ShowDialog();
    }


    private void fmMain_MouseDown(object sender, MouseButtonEventArgs e)
    {
        e.Handled = true; //double click  
        if (e.ClickCount > 1)
        {
            MenuItemShowRulers_Click(this, null!);
        }

        // fmSlider.Size = fmSlider.Size - 1;

        if (e.ChangedButton == MouseButton.Left)
        {
            FmMain.DragMove();
        }

        if (e.ChangedButton == MouseButton.Middle)
        {
            if (_isFolded)
            {
                UnFoldMainWindow();
            }
            else
            {
                FoldMainWindow();
            }
        }
    }

    private void UpdateOpacity()
    {
        if (_isMouseOver || _isWindowFocused)
            AnimateOpacity(1.0); // Fully visible
        else
            AnimateOpacity(App.Settings.Opacity); // Use custom opacity
    }

    private void fmMain_MouseEnter(object sender, MouseEventArgs e)
    {
        _isMouseOver = true;
        AnimateOpacity(1.0); // Fully visible
    }

    private void fmMain_MouseLeave(object sender, MouseEventArgs e)
    {
        _isMouseOver = false;
        UpdateOpacity();
    }

    private void AnimateOpacity(double toOpacity)
    {
        var animation = new DoubleAnimation
        {
            To = toOpacity,
            Duration = TimeSpan.FromMilliseconds(300), // adjust speed as needed
            EasingFunction = new QuadraticEase { EasingMode = EasingMode.EaseInOut }
        };

        BeginAnimation(OpacityProperty, animation);
    }

    private void FoldMainWindow()
    {
        AnimateWindowHeight(FoldedHeight);
        _isFolded = true;
    }

    private void UnFoldMainWindow()
    {
        AnimateWindowHeight(UnfoldedHeight);
        _isFolded = false;
    }

    private void AnimateWindowHeight(double toHeight)
    {
        var animation = new DoubleAnimation
        {
            To = toHeight,
            Duration = TimeSpan.FromMilliseconds(200),
            EasingFunction = new QuadraticEase { EasingMode = EasingMode.EaseInOut }
        };

        animation.Completed += (s, e) => AdjustTimeCheckerPosition();

        // Create a clock to track the animation frame-by-frame
        var animClock = animation.CreateClock();
        ApplyAnimationClock(HeightProperty, animClock);

        animClock.CurrentTimeInvalidated += (s, e) =>
        {
            // Each tick of the animation, update attached windows
            _rulers.Left = Left;
            _rulers.Top = Top + Height;

            Slider.Left = Left;
            Slider.Top = Top + Height + _rulers.Height;
        };
    }

    private void ForceToTopmost()
    { 
        if (!App.Settings.TopMost) 
            return;
        // Get the window handle
        var windowHandle = new WindowInteropHelper(this).Handle;

        // Set window to topmost position without activating it
        SetWindowPos(windowHandle,
            (IntPtr)HwndTopmost,
            0, 0, 0, 0,
            SwpNomove | SwpNosize | SwpNoactivate);
    }

    private void FmMainWindow_OnLostFocus(object sender, RoutedEventArgs e)
    {
        AnimateOpacity(App.Settings.Opacity);
    }
}
// _lstClock.Add(new OsClock("NYK", "Eastern Standard Time", "#FFAAAAFF", _lstClock.Count * 100));
// _lstClock.Add(new OsClock("LDN", "GMT Standard Time", "#FFAAFFAA", _lstClock.Count * 100));
// _lstClock.Add(new OsClock("KYIV", "FLE Standard Time", "#FFAAFFFF", _lstClock.Count * 100));
// _lstClock.Add(new OsClock("PUN", "India Standard Time", "#FF99BBBB", _lstClock.Count * 100));
// _lstClock.Add(new OsClock("SGP", "Singapore Standard Time", "#FFFFFFAA", _lstClock.Count * 100));
// _lstClock.Add(new OsClock("TKO", "Tokyo Standard Time", "#FFFFAAAA", _lstClock.Count * 100));

//I have another two window which are attached (connected to bottom side) to the main window. Now  folding is applied to the main window and once it's finished, attached windows change their position. Is it possible to move attached